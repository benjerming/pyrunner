use crate::error::Result;
use crate::ipc::{MessageReceiver, create_message_channel};
use std::thread;
use tracing::error;

pub use crate::ipc::{ConsoleProgressListener, MessageListener};
pub use crate::task_executor::TaskExecutor;

pub fn run_task_with_monitoring(
    task_id: u64,
    executor: crate::task_executor::TaskExecutor,
    listeners: Vec<Box<dyn MessageListener>>,
) -> Result<()> {
    let (sender, receiver) = create_message_channel();
    let mut message_receiver = MessageReceiver::new(receiver);

    for listener in listeners {
        message_receiver.add_listener(listener);
    }

    let monitor_handle = thread::spawn(move || {
        message_receiver.start_listening();
    });

    let result = executor.execute(task_id, &sender);

    if let Err(e) = monitor_handle.join() {
        error!("监听器线程失败: {:?}", e);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_task_with_monitoring() {
        let task_fn = |_sender: &crate::ipc::MessageSender, _task_id: u64| -> Result<()> {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        };

        let executor = crate::task_executor::TaskExecutor::new_thread(task_fn);
        let listeners = vec![
            Box::new(ConsoleProgressListener::new(1, tracing::Span::current()))
                as Box<dyn MessageListener>,
        ];

        let result = run_task_with_monitoring(1, executor, listeners);

        assert!(result.is_ok());
    }
}
