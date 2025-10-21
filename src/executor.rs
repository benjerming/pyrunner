use crate::error::{PyRunnerError, Result};
use crate::ipc::{MessageListener, MessageSender, create_message_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use tracing::{Span, error, info, info_span, instrument};

pub enum TaskExecutor {
    Thread(Arc<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>),

    Process(Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>),
}

impl std::fmt::Debug for TaskExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Thread(_) => {
                write!(f, "TaskExecutor::Thread")
            }
            Self::Process(_) => {
                write!(f, "TaskExecutor::Process")
            }
        }
    }
}

impl TaskExecutor {
    pub fn new_thread<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        Self::Thread(Arc::new(task_function))
    }

    pub fn new_process<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        Self::Process(Box::new(task_function))
    }

    pub fn execute(&self, task_id: u64, sender: &MessageSender) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.execute_async(task_id, sender))
    }

    pub async fn execute_async(&self, task_id: u64, sender: &MessageSender) -> Result<()> {
        match self {
            Self::Thread(task_function) => {
                self.execute_thread(task_id, sender, task_function).await
            }
            Self::Process(task_function) => {
                self.execute_process(task_id, sender, task_function).await
            }
        }
    }

    #[instrument(skip(self, sender, task_function))]
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

        let result = tokio::task::spawn_blocking(move || {
            match task_function(&sender_clone, task_id_clone) {
                Ok(()) => {
                    sender_clone.send_task_completed(task_id_clone);
                    info!("线程任务执行成功: {}", task_id_clone);
                    Ok(())
                }
                Err(e) => {
                    let msg = format!("线程任务执行失败: {}", e);
                    sender_clone.send_task_error_msg(task_id_clone, msg.clone());
                    error!("线程任务执行失败: {} - {}", task_id_clone, msg);
                    Err(e)
                }
            }
        })
        .await;

        match result {
            Ok(task_result) => task_result,
            Err(join_error) => {
                let msg = format!("线程执行失败: {}", join_error);
                sender.send_task_error_msg(task_id, msg.clone());
                Err(PyRunnerError::task_execution_failed(msg))
            }
        }
    }

    #[instrument(skip(self, sender, task_function))]
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
            use tracing::warn;

            warn!("Windows系统不支持fork，使用线程模拟子进程执行");
            self.execute_with_thread(task_id, sender, task_function)
                .await
        }
    }

    #[cfg(unix)]
    async fn execute_with_fork(
        &self,
        task_id: u64,
        sender: &MessageSender,
        task_function: &Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>,
    ) -> Result<()> {
        use nix::sys::wait::{WaitStatus, waitpid};
        use nix::unistd::{ForkResult, fork, getpid};
        use std::process;

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                info!(
                    "task_id: {task_id}, for成功 当前父进程PID: {}, 子进程PID: {child}",
                    getpid()
                );

                match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_, 0)) => {
                        info!("task_id: {task_id} 父进程回收子进程 {child} 完成");
                        Ok(())
                    }
                    Ok(WaitStatus::Exited(_, exit_code)) => {
                        error!("task_id: {task_id} 父进程检测到子进程失败退出码: {exit_code}");
                        let msg =
                            format!("task_id: {task_id} 父进程检测到子进程失败退出码: {exit_code}");
                        let error = PyRunnerError::task_execution_failed(msg);
                        sender.send_task_error(task_id, &error);
                        Err(error)
                    }
                    Ok(WaitStatus::Signaled(_, signal, _)) => {
                        error!("task_id: {task_id} 父进程检测到子进程被信号终止: {signal}");
                        let msg =
                            format!("task_id: {task_id} 父进程检测到子进程被信号终止: {signal}");
                        let error = PyRunnerError::task_execution_failed(msg);
                        sender.send_task_error(task_id, &error);
                        Err(error)
                    }
                    Ok(wait_status) => {
                        error!("task_id: {task_id} 父进程WaitStatus: {wait_status:?}");
                        let msg = format!("task_id: {task_id} 父进程WaitStatus: {wait_status:?}");
                        let error = PyRunnerError::task_execution_failed(msg);
                        sender.send_task_error(task_id, &error);
                        Err(error)
                    }
                    Err(e) => {
                        error!("task_id: {task_id} 回收子进程失败: {e}");
                        let msg = format!("task_id: {task_id} 回收子进程失败: {e}");
                        let error = PyRunnerError::task_execution_failed(msg);
                        sender.send_task_error(task_id, &error);
                        Err(error)
                    }
                }
            }
            Ok(ForkResult::Child) => {
                info!("task_id: {task_id} 子进程创建成功");

                let exit_code = match task_function(sender, task_id) {
                    Ok(()) => 0,
                    Err(e) => {
                        error!("task_id: {task_id} 子进程任务执行失败: {e}");
                        let msg = format!("task_id: {task_id} 子进程任务执行失败: {e}");
                        let error = PyRunnerError::task_execution_failed(msg);
                        sender.send_task_error(task_id, &error);
                        1
                    }
                };

                info!("task_id: {task_id} 子进程结束 退出码: {exit_code}");
                process::exit(exit_code);
            }
            Err(e) => {
                error!("task_id: {task_id} fork失败: {e}");
                let msg = format!("task_id: {task_id} fork失败: {e}");
                let error = PyRunnerError::task_execution_failed(msg);
                sender.send_task_error(task_id, &error);
                Err(error)
            }
        }
    }

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
                info!("task_id: {task_id} 线程任务执行成功");
                Ok(())
            }
            Err(e) => {
                error!("task_id: {task_id} 线程任务执行失败: {e}");
                let msg = format!("task_id: {task_id} 线程任务执行失败: {e}");
                let error = PyRunnerError::task_execution_failed(msg);
                sender.send_task_error(task_id, &error);
                Err(error)
            }
        }
    }

    pub async fn run_with_monitoring(
        &self,
        task_id: u64,
        listener: Arc<Mutex<dyn MessageListener + Send + Sync + 'static>>,
    ) -> Result<()> {
        let (sender, receiver) = create_message_channel(listener);

        let parent_span = Span::current();
        let monitor_handle = tokio::task::spawn_blocking(move || {
            parent_span.in_scope(|| {
                receiver.start_listening();
            });
        });

        match self.execute_async(task_id, &sender).await {
            Ok(()) => info!("任务执行成功"),
            Err(e) => {
                let msg = format!("任务执行失败: {e:?}");
                error!("任务执行失败: {e:?}");
                return Err(PyRunnerError::task_execution_failed(msg));
            }
        }
        info!("关闭发送器连接");
        drop(sender);

        match monitor_handle.await {
            Ok(()) => info!("回收监听器线程成功"),
            Err(e) => {
                let msg = format!("回收监听器线程失败: {e:?}");
                error!("回收监听器线程失败: {e:?}");
                return Err(PyRunnerError::task_execution_failed(msg));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{ConsoleProgressListener, create_message_channel};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use tracing::debug;

    #[test]
    fn test_thread_task_executor() {
        let listener = Arc::new(Mutex::new(ConsoleProgressListener::new(
            1,
            tracing::Span::current(),
        )));
        let (sender, receiver) = create_message_channel(listener);

        let task_fn = |sender: &MessageSender, task_id: u64| -> Result<()> {
            use std::thread;
            use std::time::Duration;

            for i in 1..=5 {
                thread::sleep(Duration::from_millis(200));
                sender.send_task_progress(task_id, i, 5);
                debug!("执行步骤 {}/5", i);
            }

            debug!("任务执行成功");
            Ok(())
        };

        let executor = TaskExecutor::new_thread(task_fn);

        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        thread::spawn(move || {
            loop {
                match receiver.try_recv_timeout(Duration::from_secs(3)) {
                    Ok(msg) => {
                        info!("收到消息: {:?}", msg);
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
        assert!(messages.len() >= 5);
    }

    #[test]
    fn test_process_task_executor() {
        let listener = Arc::new(Mutex::new(ConsoleProgressListener::new(
            2,
            tracing::Span::current(),
        )));

        let task_fn = |_sender: &MessageSender, _task_id: u64| -> Result<()> {
            use std::thread;
            use std::time::Duration;

            for i in 1..=3 {
                thread::sleep(Duration::from_millis(50));
                info!("执行步骤 {}/3", i);
            }

            info!("任务执行成功");
            Ok(())
        };

        let executor = TaskExecutor::new_process(task_fn);

        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        let result = rt.block_on(executor.run_with_monitoring(2, listener));

        assert!(result.is_ok());
    }
}
