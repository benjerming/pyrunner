use super::message::Message;
use super::sender::MessageSender;
use ipc_channel::ipc::{self, IpcReceiver};

pub fn create_message_channel() -> (MessageSender, IpcReceiver<Message>) {
    let (sender, receiver) = ipc::channel().expect("Failed to create IPC channel");
    let message_sender = MessageSender::new(sender);
    (message_sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::super::message::ProgressMessage;
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_create_channel() {
        let (sender, receiver) = create_message_channel();

        let progress = ProgressMessage::new(1);
        sender.send_progress_safe(progress);

        let msg = receiver
            .try_recv_timeout(Duration::from_millis(100))
            .unwrap();
        if let Message::Progress(p) = msg {
            assert_eq!(p.task_id, 1);
        } else {
            panic!("期望收到 Progress 消息");
        }
    }
}
