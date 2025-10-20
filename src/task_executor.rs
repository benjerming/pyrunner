use crate::error::{PyRunnerError, Result};
use crate::message_sender::MessageSender;
use log::{error, info, warn};
use std::sync::Arc;

pub trait TaskExecutor: Send + Sync {
    fn execute(&self, task_id: String, sender: &MessageSender) -> Result<()>;
}

pub struct ThreadTaskExecutor {
    task_function: Arc<dyn Fn(&MessageSender) -> Result<()> + Send + Sync>,
}

impl ThreadTaskExecutor {
    pub fn new<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            task_function: Arc::new(task_function),
        }
    }

    pub async fn execute_async(&self, task_id: String, sender: &MessageSender) -> Result<()> {
        info!("开始通过线程执行任务 (任务ID: {})", task_id);

        let task_function = self.task_function.clone();
        let task_id_clone = task_id.clone();
        let sender_clone = sender.clone();

        let result = tokio::task::spawn_blocking(move || match task_function(&sender_clone) {
            Ok(()) => {
                sender_clone.send_task_completed(task_id_clone.clone());
                info!("线程任务执行成功: {}", task_id_clone);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("线程任务执行失败: {}", e);
                sender_clone.send_task_error(task_id_clone.clone(), error_msg.clone());
                error!("线程任务执行失败: {} - {}", task_id_clone, error_msg);
                Err(e)
            }
        })
        .await;

        match result {
            Ok(task_result) => task_result,
            Err(join_error) => {
                let error_msg = format!("线程执行失败: {}", join_error);
                sender.send_task_error(task_id, error_msg.clone());
                Err(PyRunnerError::task_execution_failed(error_msg))
            }
        }
    }
}

impl TaskExecutor for ThreadTaskExecutor {
    fn execute(&self, task_id: String, sender: &MessageSender) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.execute_async(task_id, sender))
    }
}

pub struct ProcessTaskExecutor {
    task_function: Box<dyn Fn(&MessageSender) -> Result<()> + Send + Sync>,
}

impl ProcessTaskExecutor {
    pub fn new<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            task_function: Box::new(task_function),
        }
    }

    pub async fn execute_async(&self, task_id: String, sender: &MessageSender) -> Result<()> {
        info!("开始执行任务 (任务ID: {})", task_id);

        #[cfg(unix)]
        {
            self.execute_with_fork(task_id, sender).await
        }

        #[cfg(windows)]
        {
            warn!("Windows系统不支持fork，使用线程模拟子进程执行");
            self.execute_with_thread(task_id, sender).await
        }
    }

    #[cfg(unix)]
    async fn execute_with_fork(&self, task_id: String, sender: &MessageSender) -> Result<()> {
        use nix::sys::wait::{WaitStatus, waitpid};
        use nix::unistd::{ForkResult, fork};
        use std::process;

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // 父进程
                info!("父进程等待子进程完成，子进程PID: {}", child);

                match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_, exit_code)) => {
                        if exit_code == 0 {
                            info!("子进程任务执行成功: {}", task_id);
                            Ok(())
                        } else {
                            let error_msg = format!("子进程执行失败，退出码: {}", exit_code);
                            sender.send_task_error(task_id.clone(), error_msg.clone());
                            error!("子进程任务执行失败: {} - {}", task_id, error_msg);
                            Err(PyRunnerError::task_execution_failed(error_msg))
                        }
                    }
                    Ok(WaitStatus::Signaled(_, signal, _)) => {
                        let error_msg = format!("子进程被信号终止: {:?}", signal);
                        sender.send_task_error(task_id.clone(), error_msg.clone());
                        warn!("子进程被信号终止: {} - {}", task_id, error_msg);
                        Ok(())
                    }
                    Ok(status) => {
                        let error_msg = format!("子进程异常终止: {:?}", status);
                        sender.send_task_error(task_id.clone(), error_msg.clone());
                        error!("子进程异常终止: {} - {}", task_id, error_msg);
                        Err(PyRunnerError::task_execution_failed(error_msg))
                    }
                    Err(e) => {
                        let error_msg = format!("等待子进程失败: {}", e);
                        sender.send_task_error(task_id.clone(), error_msg.clone());
                        Err(PyRunnerError::task_execution_failed(error_msg))
                    }
                }
            }
            Ok(ForkResult::Child) => {
                info!("在子进程中执行任务: {}", task_id);

                let exit_code = match (self.task_function)(sender) {
                    Ok(()) => {
                        info!("子进程任务执行成功");
                        0
                    }
                    Err(e) => {
                        error!("子进程任务执行失败: {}", e);
                        sender.send_task_error(task_id.clone(), e.to_string());
                        1
                    }
                };

                process::exit(exit_code);
            }
            Err(e) => {
                let error_msg = format!("fork失败: {}", e);
                sender.send_task_error(task_id.clone(), error_msg.clone());
                Err(PyRunnerError::task_execution_failed(error_msg))
            }
        }
    }

    #[cfg(windows)]
    async fn execute_with_thread(&self, task_id: String, sender: &MessageSender) -> Result<()> {
        match (self.task_function)(sender) {
            Ok(()) => {
                sender.send_task_completed(task_id.clone());
                info!("线程任务执行成功: {}", task_id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("线程任务执行失败: {}", e);
                sender.send_task_error(task_id.clone(), error_msg.clone());
                error!("线程任务执行失败: {} - {}", task_id, error_msg);
                Err(e)
            }
        }
    }
}

impl TaskExecutor for ProcessTaskExecutor {
    fn execute(&self, task_id: String, sender: &MessageSender) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.execute_async(task_id, sender))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_sender::create_message_channel;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_thread_task_executor() {
        let (sender, receiver) = create_message_channel();

        let task_fn = |sender: &MessageSender| -> Result<()> {
            use std::thread;
            use std::time::Duration;

            for i in 1..=5 {
                thread::sleep(Duration::from_millis(200));
                let percentage = (i as f64 / 5.0) * 100.0;
                sender.send_task_progress(
                    "process_task".to_string(),
                    percentage,
                    format!("执行步骤 {}/5", i),
                );
                println!("执行步骤 {}/5", i);
            }

            println!("任务执行成功");
            Ok(())
        };

        let executor = ThreadTaskExecutor::new(task_fn);

        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        thread::spawn(move || {
            loop {
                match receiver.recv_timeout(Duration::from_secs(3)) {
                    Ok(msg) => {
                        println!("收到消息: {:?}", msg);
                        let mut msgs = messages_clone.lock().unwrap();
                        msgs.push(msg);
                    }
                    Err(e) => {
                        error!("接收消息失败: {}", e);
                        break;
                    }
                }
            }
        });

        let result = executor.execute("test_thread".to_string(), &sender);
        assert!(result.is_ok());

        let messages = messages.lock().unwrap();
        assert!(messages.len() >= 5); // 测试用例发送5条消息
    }

    #[test]
    fn test_process_task_executor() {
        let (sender, receiver) = create_message_channel();

        let task_fn = |sender: &MessageSender| -> Result<()> {
            use std::thread;
            use std::time::Duration;

            for i in 1..=5 {
                thread::sleep(Duration::from_millis(200));
                let percentage = (i as f64 / 5.0) * 100.0;
                sender.send_task_progress(
                    "process_task".to_string(),
                    percentage,
                    format!("执行步骤 {}/5", i),
                );
                println!("执行步骤 {}/5", i);
            }

            println!("任务执行成功");
            Ok(())
        };

        let executor = ProcessTaskExecutor::new(task_fn);

        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        // 在另一个线程中收集消息
        thread::spawn(move || {
            loop {
                match receiver.recv_timeout(Duration::from_secs(3)) {
                    Ok(msg) => {
                        println!("收到消息: {:?}", msg);
                        let mut msgs = messages_clone.lock().unwrap();
                        msgs.push(msg);
                    }
                    Err(e) => {
                        error!("接收消息失败: {}", e);
                        break;
                    }
                }
            }
        });

        // 执行任务
        let result = executor.execute("test_process".to_string(), &sender);
        assert!(result.is_ok());

        // 等待消息收集完成
        thread::sleep(Duration::from_millis(200));

        let messages = messages.lock().unwrap();
        assert!(messages.len() >= 5); // 测试用例发送5条消息
    }
}
