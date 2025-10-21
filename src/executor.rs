use crate::error::{PyRunnerError, Result};
use crate::ipc::{MessageListener, MessageSender, create_message_channel};
use std::sync::{Arc, Mutex};
use tracing::{Span, error, info, instrument};

pub enum TaskExecutor {
    Thread(Arc<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>),

    Process(Box<dyn Fn(&MessageSender, u64) -> Result<()> + Send + Sync>),
}

impl TaskExecutor {
    pub fn new_thread<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        Self::Thread(Arc::new(task_function))
    }

    #[cfg(unix)]
    pub fn new_process<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        Self::Process(Box::new(task_function))
    }

    #[cfg(windows)]
    pub fn new_process<F>(task_function: F) -> Self
    where
        F: Fn(&MessageSender, u64) -> Result<()> + Send + Sync + 'static,
    {
        warn!("Windows系统不支持fork，使用线程模拟子进程执行");
        Self::Thread(Arc::new(task_function))
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
    use crate::ipc::{ErrorMessage, ProgressMessage, ResultMessage};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct TestListener {
        progress_count: u32,
        error_count: u32,
        result_count: u32,
    }

    impl MessageListener for TestListener {
        fn on_progress(&mut self, _progress: &ProgressMessage) {
            self.progress_count += 1;
        }
        fn on_error(&mut self, _error: &ErrorMessage) {
            self.error_count += 1;
        }
        fn on_result(&mut self, _result: &ResultMessage) {
            self.result_count += 1;
        }
    }

    #[test]
    fn test_thread_task_executor() {
        let task_id = 1;
        let listener = Arc::new(Mutex::new(TestListener::default()));

        let task_fn = move |sender: &MessageSender, _task_id: u64| -> Result<()> {
            assert_eq!(_task_id, task_id);
            sender.send_task_progress(task_id, 1, 1);
            sender.send_task_completed(task_id);
            sender.send_task_error_msg(task_id, "test error".to_string());
            Ok(())
        };

        let executor = TaskExecutor::new_thread(task_fn);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(executor.run_with_monitoring(1, listener.clone()));
        assert!(result.is_ok());

        let guard = listener.lock().unwrap();
        assert_eq!(guard.progress_count, 1);
        assert_eq!(guard.error_count, 1);
        assert_eq!(guard.result_count, 1);
    }

    #[test]
    fn test_process_task_executor() {
        let task_id = 2;
        let listener = Arc::new(Mutex::new(TestListener::default()));

        let task_fn = move |sender: &MessageSender, _task_id: u64| -> Result<()> {
            assert_eq!(_task_id, task_id);
            sender.send_task_progress(task_id, 1, 1);
            sender.send_task_completed(task_id);
            sender.send_task_error_msg(task_id, "test error".to_string());
            Ok(())
        };

        let executor = TaskExecutor::new_process(task_fn);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(executor.run_with_monitoring(2, listener.clone()));

        assert!(result.is_ok());
        let guard = listener.lock().unwrap();

        assert_eq!(guard.progress_count, 1);
        assert_eq!(guard.error_count, 1);
        assert_eq!(guard.result_count, 1);
    }
}
