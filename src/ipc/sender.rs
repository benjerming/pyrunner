use super::message::{ErrorMessage, Message, ProgressMessage, ResultMessage};
use crate::error::PyRunnerError;
use ipc_channel::ipc::IpcSender;
use tracing::{debug, error};

#[derive(Clone)]
pub struct MessageSender {
    sender: IpcSender<Message>,
}

impl MessageSender {
    pub fn new(sender: IpcSender<Message>) -> Self {
        Self { sender }
    }

    pub fn send(&self, message: Message) -> Result<(), bincode::Error> {
        debug!("发送消息: {:?}", message);
        self.sender.send(message).map_err(|e| {
            bincode::Error::new(bincode::ErrorKind::Custom(format!(
                "IPC send error: {:?}",
                e
            )))
        })
    }

    pub fn send_safe(&self, message: Message) {
        let _ = self.send(message).inspect_err(|e| {
            error!("发送消息失败: {e:?}");
        });
    }

    pub fn send_progress_safe(&self, progress: ProgressMessage) {
        self.send_safe(Message::Progress(progress));
    }

    pub fn send_error_safe(&self, error_info: ErrorMessage) {
        self.send_safe(Message::Error(error_info));
    }

    pub fn send_result_safe(&self, result_info: ResultMessage) {
        self.send_safe(Message::Result(result_info));
    }

    #[allow(dead_code)]
    pub fn send_task_started(&self, task_id: u64) {
        let progress = ProgressMessage::new(task_id);
        self.send_progress_safe(progress);
    }

    pub fn send_task_progress(&self, task_id: u64, done: u64, size: u64) {
        let mut progress = ProgressMessage::new(task_id);
        progress.update_progress(done, size);
        self.send_progress_safe(progress);
    }

    pub fn send_task_completed(&self, task_id: u64) {
        let result = ResultMessage::new(task_id, 0, 0);
        self.send_result_safe(result);
    }

    pub fn send_task_error_msg(&self, task_id: u64, error_msg: String) {
        let error = ErrorMessage::from_string(task_id, error_msg);
        self.send_error_safe(error);
    }

    pub fn send_task_error(&self, task_id: u64, error: &PyRunnerError) {
        let error_info = ErrorMessage::new(task_id, error);
        self.send_error_safe(error_info);
    }

    #[allow(dead_code)]
    pub fn get_raw_sender(&self) -> IpcSender<Message> {
        self.sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::create_message_channel;
    use std::time::Duration;

    #[test]
    fn test_message_sender() {
        let (sender, receiver) = create_message_channel();

        sender.send_task_started(1);
        sender.send_task_progress(1, 50, 100);
        sender.send_task_completed(1);

        let msg1 = receiver
            .try_recv_timeout(Duration::from_millis(100))
            .unwrap();
        if let Message::Progress(progress) = msg1 {
            assert_eq!(progress.task_id, 1);
            assert_eq!(progress.done, 0);
            assert_eq!(progress.size, 0);
        } else {
            panic!("期望收到 Progress 消息");
        }

        let msg2 = receiver
            .try_recv_timeout(Duration::from_millis(100))
            .unwrap();
        if let Message::Progress(progress) = msg2 {
            assert_eq!(progress.done, 50);
            assert_eq!(progress.size, 100);
        } else {
            panic!("期望收到 Progress 消息");
        }

        let msg3 = receiver
            .try_recv_timeout(Duration::from_millis(100))
            .unwrap();
        if let Message::Result(result) = msg3 {
            assert_eq!(result.task_id, 1);
        } else {
            panic!("期望收到 Result 消息");
        }
    }

    #[test]
    fn test_error_handling() {
        let (sender, receiver) = create_message_channel();

        sender.send_task_error_msg(1, "模拟错误".to_string());

        let msg = receiver
            .try_recv_timeout(Duration::from_millis(100))
            .unwrap();
        if let Message::Error(error) = msg {
            assert_eq!(error.task_id, 1);
            assert_eq!(error.error_message, "模拟错误");
        } else {
            panic!("期望收到 Error 消息");
        }
    }

    #[test]
    fn test_error_message() {
        let (sender, receiver) = create_message_channel();

        let error = PyRunnerError::task_execution_failed("测试错误");
        sender.send_task_error(1, &error);

        let msg = receiver
            .try_recv_timeout(Duration::from_millis(100))
            .unwrap();
        if let Message::Error(error_info) = msg {
            assert_eq!(error_info.task_id, 1);
            assert_eq!(error_info.error_code, 1001);
            assert!(error_info.error_message.contains("测试错误"));
        } else {
            panic!("期望收到 Error 消息");
        }
    }
}
