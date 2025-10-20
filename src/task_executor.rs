use crate::error::{PyRunnerError, Result};
use crate::ipc::MessageSender;
use tracing::{error, info, warn};
use std::sync::Arc;

/// 任务执行器，支持线程和进程两种执行模式
pub enum TaskExecutor {
    /// 线程模式：使用tokio线程池执行任务
    Thread {
        task_function: Arc<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    },
    /// 进程模式：在Unix上使用fork创建子进程，在Windows上使用线程模拟
    Process {
        task_function: Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    },
}

impl TaskExecutor {
    /// 创建一个线程模式的任务执行器
    pub fn new_thread<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        Self::Thread {
            task_function: Arc::new(task_function),
        }
    }

    /// 创建一个进程模式的任务执行器
    pub fn new_process<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        Self::Process {
            task_function: Box::new(task_function),
        }
    }

    /// 执行任务（同步接口）
    pub fn execute(&self, task_id: u64, sender: &MessageSender) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.execute_async(task_id, sender))
    }

    /// 执行任务（异步接口）
    pub async fn execute_async(&self, task_id: u64, sender: &MessageSender) -> Result<()> {
        match self {
            Self::Thread { task_function } => {
                self.execute_thread(task_id, sender, task_function).await
            }
            Self::Process { task_function } => {
                self.execute_process(task_id, sender, task_function).await
            }
        }
    }

    /// 线程模式执行
    async fn execute_thread(
        &self,
        task_id: u64,
        sender: &MessageSender,
        task_function: &Arc<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    ) -> Result<()> {
        info!("开始通过线程执行任务 (任务ID: {})", task_id);

        let task_function = task_function.clone();
        let task_id_clone = task_id;
        let sender_clone = sender.clone();

        let result = tokio::task::spawn_blocking(move || match task_function(&sender_clone, task_id_clone) {
            Ok(()) => {
                sender_clone.send_task_completed(task_id_clone);
                info!("线程任务执行成功: {}", task_id_clone);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("线程任务执行失败: {}", e);
                sender_clone.send_task_error(task_id_clone, error_msg.clone());
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

    /// 进程模式执行
    async fn execute_process(
        &self,
        task_id: u64,
        sender: &MessageSender,
        task_function: &Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    ) -> Result<()> {
        info!("开始执行任务 (任务ID: {})", task_id);

        #[cfg(unix)]
        {
            self.execute_with_fork(task_id, sender, task_function).await
        }

        #[cfg(windows)]
        {
            warn!("Windows系统不支持fork，使用线程模拟子进程执行");
            self.execute_with_thread(task_id, sender, task_function)
                .await
        }
    }

    /// Unix系统下使用fork执行
    #[cfg(unix)]
    async fn execute_with_fork(
        &self,
        task_id: u64,
        sender: &MessageSender,
        task_function: &Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    ) -> Result<()> {
        use nix::sys::wait::{WaitStatus, waitpid};
        use nix::unistd::{ForkResult, fork};
        use std::process;

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // 父进程
                info!("execute_with_fork: {} 父进程等待子进程完成，子进程PID: {}", task_id, child);

                match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_, exit_code)) => {
                        if exit_code == 0 {
                            // info!("子进程任务执行成功: {}", task_id);
                            // 父进程在子进程完成后发送完成消息
                            // sender.send_task_completed(task_id);
                            Ok(())
                        } else {
                            let error_msg = format!("子进程执行失败，退出码: {}", exit_code);
                            sender.send_task_error(task_id, error_msg.clone());
                            error!("execute_with_fork: 子进程任务执行失败 (任务ID: {}): {}", task_id, error_msg);
                            Err(PyRunnerError::task_execution_failed(error_msg))
                        }
                    }
                    Ok(WaitStatus::Signaled(_, signal, _)) => {
                        let error_msg = format!("子进程被信号终止: {:?}", signal);
                        sender.send_task_error(task_id, error_msg.clone());
                        warn!("execute_with_fork: 子进程被信号终止 (任务ID: {}): {}", task_id, error_msg);
                        Ok(())
                    }
                    Ok(status) => {
                        let error_msg = format!("子进程异常终止: {:?}", status);
                        sender.send_task_error(task_id, error_msg.clone());
                        error!("execute_with_fork: 子进程异常终止 (任务ID: {}): {}", task_id, error_msg);
                        Err(PyRunnerError::task_execution_failed(error_msg))
                    }
                    Err(e) => {
                        let error_msg = format!("等待子进程失败: {}", e);
                        sender.send_task_error(task_id, error_msg.clone());
                        Err(PyRunnerError::task_execution_failed(error_msg))
                    }
                }
            }
            Ok(ForkResult::Child) => {
                info!("在子进程中执行任务: {}", task_id);

                let exit_code = match task_function(sender, task_id) {
                    Ok(()) => {
                        info!("execute_with_fork: {} 子进程任务执行成功", task_id);
                        // 注意：不在子进程中发送完成消息，因为父进程会在等待完成后发送
                        0
                    }
                    Err(e) => {
                        error!("子进程任务执行失败: {}", e);
                        sender.send_task_error(task_id, e.to_string());
                        1
                    }
                };

                process::exit(exit_code);
            }
            Err(e) => {
                let error_msg = format!("fork失败: {}", e);
                sender.send_task_error(task_id, error_msg.clone());
                Err(PyRunnerError::task_execution_failed(error_msg))
            }
        }
    }

    /// Windows系统下使用线程模拟进程执行
    #[cfg(windows)]
    async fn execute_with_thread(
        &self,
        task_id: u64,
        sender: &MessageSender,
        task_function: &Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    ) -> Result<()> {
        match task_function(sender, task_id) {
            Ok(()) => {
                sender.send_task_completed(task_id);
                info!("线程任务执行成功: {}", task_id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("线程任务执行失败: {}", e);
                sender.send_task_error(task_id, error_msg.clone());
                error!("线程任务执行失败: {} - {}", task_id, error_msg);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::create_message_channel;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_thread_task_executor() {
        let (sender, receiver) = create_message_channel();

        let task_fn = |sender: &MessageSender, task_id: u64| -> Result<()> {
            use std::thread;
            use std::time::Duration;

            for i in 1..=5 {
                thread::sleep(Duration::from_millis(200));
                sender.send_task_progress(task_id, i, 5);
                println!("执行步骤 {}/5", i);
            }

            println!("任务执行成功");
            Ok(())
        };

        let executor = TaskExecutor::new_thread(task_fn);

        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        thread::spawn(move || {
            loop {
                match receiver.try_recv_timeout(Duration::from_secs(3)) {
                    Ok(msg) => {
                        println!("收到消息: {:?}", msg);
                        let mut msgs = messages_clone.lock().unwrap();
                        msgs.push(msg);
                    }
                    Err(e) => {
                        error!("接收消息失败: {:?}", e);
                        break;
                    }
                }
            }
        });

        let result = executor.execute(1, &sender);
        assert!(result.is_ok());

        let messages = messages.lock().unwrap();
        assert!(messages.len() >= 5); // 测试用例发送5条消息
    }

    #[test]
    fn test_process_task_executor() {
        let (sender, _receiver) = create_message_channel();

        let task_fn = |_sender: &MessageSender, _task_id: u64| -> Result<()> {
            use std::thread;
            use std::time::Duration;

            // 在子进程或线程中执行简单任务
            for i in 1..=3 {
                thread::sleep(Duration::from_millis(50));
                println!("执行步骤 {}/3", i);
            }

            println!("任务执行成功");
            Ok(())
        };

        let executor = TaskExecutor::new_process(task_fn);

        // 执行任务 - 在Unix上会fork子进程，在Windows上会使用线程
        let result = executor.execute(1, &sender);
        
        // 验证任务执行成功
        assert!(result.is_ok());
    }
}
