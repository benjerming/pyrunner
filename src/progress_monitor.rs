use crate::message_receiver::MessageReceiver;
use crate::message_sender::{MessageSender, create_message_channel};
use crate::task_executor::TaskExecutor;
use log::error;
use std::thread;

// 重新导出核心类型，保持向后兼容
pub use crate::message_receiver::{ConsoleProgressListener, ProgressListener};
pub use crate::message_sender::ProgressInfo;
pub use crate::task_executor::{ThreadTaskExecutor, ProcessTaskExecutor};

/// 进度监控器 - 高级封装，整合消息发送器、接收器和任务执行器
pub struct ProgressMonitor {
    message_sender: MessageSender,
    message_receiver: MessageReceiver,
}

impl ProgressMonitor {
    /// 创建新的进度监控器
    pub fn new() -> Self {
        let (sender, receiver) = create_message_channel();
        let message_receiver = MessageReceiver::new(receiver);

        Self {
            message_sender: sender,
            message_receiver,
        }
    }

    /// 添加进度监听器
    pub fn add_listener(&mut self, listener: Box<dyn ProgressListener>) {
        self.message_receiver.add_listener(listener);
    }

    /// 启动监听循环
    pub fn start_monitoring(&self) {
        self.message_receiver.start_listening();
    }

    /// 分离监控器，返回发送器和接收器
    pub fn split(self) -> (MessageSender, MessageReceiver) {
        (self.message_sender, self.message_receiver)
    }
}


/// 便利函数：运行任务并监控进度
pub fn run_task_with_monitoring<T: TaskExecutor>(
    task_id: String,
    executor: T,
    listeners: Vec<Box<dyn ProgressListener>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (sender, receiver) = create_message_channel();
    let mut message_receiver = MessageReceiver::new(receiver);

    // 添加监听器
    for listener in listeners {
        message_receiver.add_listener(listener);
    }

    // 在子线程中启动监听
    let monitor_handle = thread::spawn(move || {
        message_receiver.start_listening();
    });

    // 执行任务
    let result = executor.execute(task_id, &sender);

    // 等待监听器完成
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
        let executor = ThreadTaskExecutor::new(0, 3); // 快速任务
        let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>];

        let result = run_task_with_monitoring("test_task".to_string(), executor, listeners);

        assert!(result.is_ok());
    }
}
