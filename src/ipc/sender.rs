use super::message::{ErrorMessage, Message, ProgressMessage, ResultMessage};
use crate::error::PyRunnerError;
use ipc_channel::ipc::IpcSender;
use tracing::{debug, error};

#[derive(Clone)]
pub struct MessageSender {
    sender: IpcSender<Message>,
}

#[allow(dead_code)]
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
    pub fn send_task_started(&self) {
        let progress = ProgressMessage::new(0, 0);
        self.send_progress_safe(progress);
    }

    pub fn send_task_progress(&self, done: u64, size: u64) {
        let progress = ProgressMessage::new(done, size);
        self.send_progress_safe(progress);
    }

    pub fn send_task_completed(&self) {
        let result = ResultMessage::new(100, 5000);
        self.send_result_safe(result);
    }

    pub fn send_task_error_msg(&self, error_msg: String) {
        let error = ErrorMessage::new(1001, error_msg);
        self.send_error_safe(error);
    }

    pub fn send_task_error(&self, error: &PyRunnerError) {
        let error_info = ErrorMessage::from(error);
        self.send_error_safe(error_info);
    }

    #[allow(dead_code)]
    pub fn get_raw_sender(&self) -> IpcSender<Message> {
        self.sender.clone()
    }
}
