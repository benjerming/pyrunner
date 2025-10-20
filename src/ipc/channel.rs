use ipc_channel::ipc::{self, IpcReceiver};
use super::message::Message;
use super::sender::MessageSender;

/// 创建消息通道
/// 
/// 返回一个元组，包含消息发送者和接收者
pub fn create_message_channel() -> (MessageSender, IpcReceiver<Message>) {
    let (sender, receiver) = ipc::channel().expect("Failed to create IPC channel");
    let message_sender = MessageSender::new(sender);
    (message_sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::message::ProgressInfo;
    use std::time::Duration;

    #[test]
    fn test_create_channel() {
        let (sender, receiver) = create_message_channel();
        
        // 测试通道是否可用
        let progress = ProgressInfo::new(1);
        sender.send_progress(progress).unwrap();
        
        let msg = receiver.try_recv_timeout(Duration::from_millis(100)).unwrap();
        if let Message::Progress(p) = msg {
            assert_eq!(p.task_id, 1);
        } else {
            panic!("期望收到 Progress 消息");
        }
    }
}

