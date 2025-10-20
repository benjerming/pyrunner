use crate::error::Result;
use crate::message_receiver::MessageReceiver;
use crate::message_sender::{MessageSender, create_message_channel};
use crate::task_executor::TaskExecutor;
use log::error;
use std::thread;

// 导出消息相关类型
pub use crate::message_sender::{ErrorInfo, Message, ProgressInfo, ResultInfo};
pub use crate::message_receiver::{
    ConsoleMessageListener, ConsoleProgressListener, MessageListener, ProgressListener,
};
pub use crate::task_executor::{ProcessTaskExecutor, ThreadTaskExecutor};

pub struct ProgressMonitor {
    message_sender: MessageSender,
    message_receiver: MessageReceiver,
}

impl ProgressMonitor {
    pub fn new() -> Self {
        let (sender, receiver) = create_message_channel();
        let message_receiver = MessageReceiver::new(receiver);

        Self {
            message_sender: sender,
            message_receiver,
        }
    }

    pub fn add_listener(&mut self, listener: Box<dyn MessageListener>) {
        self.message_receiver.add_listener(listener);
    }

    pub fn start_monitoring(&self) {
        self.message_receiver.start_listening();
    }

    pub fn split(self) -> (MessageSender, MessageReceiver) {
        (self.message_sender, self.message_receiver)
    }
}

pub fn run_task_with_monitoring<T: TaskExecutor>(
    task_id: String,
    executor: T,
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
        let task_fn = |_sender: &crate::message_sender::MessageSender| -> Result<()> {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        };

        let executor = ThreadTaskExecutor::new(task_fn);
        let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn MessageListener>];

        let result = run_task_with_monitoring("test_task".to_string(), executor, listeners);

        assert!(result.is_ok());
    }
}
