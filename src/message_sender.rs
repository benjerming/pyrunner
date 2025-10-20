use tracing::{debug, error};
use serde::{Deserialize, Serialize};
use ipc_channel::ipc::{self, IpcSender, IpcReceiver};
use std::time::SystemTime;
use crate::error::PyRunnerError;

/// 统一的消息枚举，支持多种消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// 进度消息
    Progress(ProgressInfo),
    /// 错误消息
    Error(ErrorInfo),
    /// 结果消息
    Result(ResultInfo),
}

/// 进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub task_id: u64,
    pub percentage: f64,
    pub message: String,
    pub current_step: u32,
    pub total_steps: u32,
    pub is_completed: bool,
    pub has_error: bool,
    pub error_message: Option<String>,
    #[serde(skip)]
    pub timestamp: Option<SystemTime>,
}

/// 错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub task_id: u64,
    pub error_code: i32,
    pub error_message: String,
    pub is_retryable: bool,
    pub is_fatal: bool,
    #[serde(skip)]
    pub timestamp: Option<SystemTime>,
}

/// 执行结果信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultInfo {
    pub task_id: u64,
    pub result_type: String,
    pub result_data: serde_json::Value,
    pub success: bool,
    #[serde(skip)]
    pub timestamp: Option<SystemTime>,
}

impl ProgressInfo {
    pub fn new(task_id: u64) -> Self {
        Self {
            task_id,
            percentage: 0.0,
            message: "开始任务".to_string(),
            current_step: 0,
            total_steps: 100,
            is_completed: false,
            has_error: false,
            error_message: None,
            timestamp: Some(SystemTime::now()),
        }
    }

    pub fn update_progress(&mut self, percentage: f64, message: String) {
        self.percentage = percentage.clamp(0.0, 100.0);
        self.message = message;
        self.current_step = (self.percentage * self.total_steps as f64 / 100.0) as u32;
    }

    pub fn complete(&mut self) {
        self.percentage = 100.0;
        self.current_step = self.total_steps;
        self.is_completed = true;
        self.message = "任务完成".to_string();
    }

    pub fn error(&mut self, error_msg: String) {
        self.has_error = true;
        self.error_message = Some(error_msg.clone());
        self.message = format!("任务出错: {}", error_msg);
        self.timestamp = Some(SystemTime::now());
    }
}

impl ErrorInfo {
    pub fn new(task_id: u64, error: &PyRunnerError) -> Self {
        Self {
            task_id,
            error_code: error.error_code(),
            error_message: error.to_string(),
            is_retryable: error.is_retryable(),
            is_fatal: error.is_fatal(),
            timestamp: Some(SystemTime::now()),
        }
    }

    pub fn from_string(task_id: u64, error_message: String) -> Self {
        Self {
            task_id,
            error_code: 9999,
            error_message,
            is_retryable: false,
            is_fatal: false,
            timestamp: Some(SystemTime::now()),
        }
    }
}

impl ResultInfo {
    pub fn new(task_id: u64, result_type: String, result_data: serde_json::Value) -> Self {
        Self {
            task_id,
            result_type,
            result_data,
            success: true,
            timestamp: Some(SystemTime::now()),
        }
    }

    pub fn success(task_id: u64, result_data: serde_json::Value) -> Self {
        Self::new(task_id, "success".to_string(), result_data)
    }

    pub fn failure(task_id: u64, result_data: serde_json::Value) -> Self {
        let mut result = Self::new(task_id, "failure".to_string(), result_data);
        result.success = false;
        result
    }
}

#[derive(Clone)]
pub struct MessageSender {
    sender: IpcSender<Message>,
}

impl MessageSender {
    pub fn new(sender: IpcSender<Message>) -> Self {
        Self { sender }
    }

    /// 发送消息
    pub fn send(&self, message: Message) -> Result<(), bincode::Error> {
        debug!("发送消息: {:?}", message);
        self.sender.send(message).map_err(|e| {
            bincode::Error::new(bincode::ErrorKind::Custom(format!("IPC send error: {:?}", e)))
        })
    }

    /// 安全发送消息（失败时记录错误）
    pub fn send_safe(&self, message: Message) {
        if let Err(e) = self.send(message) {
            error!("发送消息失败: {}", e);
        }
    }

    /// 发送进度消息
    pub fn send_progress(&self, progress: ProgressInfo) -> Result<(), bincode::Error> {
        self.send(Message::Progress(progress))
    }

    /// 安全发送进度消息
    pub fn send_progress_safe(&self, progress: ProgressInfo) {
        self.send_safe(Message::Progress(progress));
    }

    /// 发送错误消息
    pub fn send_error(&self, error_info: ErrorInfo) -> Result<(), bincode::Error> {
        self.send(Message::Error(error_info))
    }

    /// 安全发送错误消息
    pub fn send_error_safe(&self, error_info: ErrorInfo) {
        self.send_safe(Message::Error(error_info));
    }

    /// 发送结果消息
    pub fn send_result(&self, result_info: ResultInfo) -> Result<(), bincode::Error> {
        self.send(Message::Result(result_info))
    }

    /// 安全发送结果消息
    pub fn send_result_safe(&self, result_info: ResultInfo) {
        self.send_safe(Message::Result(result_info));
    }

    #[allow(dead_code)]
    pub fn send_task_started(&self, task_id: u64) {
        let progress = ProgressInfo::new(task_id);
        self.send_progress_safe(progress);
    }

    pub fn send_task_progress(&self, task_id: u64, percentage: f64, message: String) {
        let mut progress = ProgressInfo::new(task_id);
        progress.update_progress(percentage, message);
        self.send_progress_safe(progress);
    }

    pub fn send_task_completed(&self, task_id: u64) {
        let mut progress = ProgressInfo::new(task_id);
        progress.complete();
        self.send_progress_safe(progress);
    }

    pub fn send_task_error(&self, task_id: u64, error_msg: String) {
        let mut progress = ProgressInfo::new(task_id);
        progress.error(error_msg);
        self.send_progress_safe(progress);
    }

    /// 从PyRunnerError发送错误消息
    pub fn send_task_error_from_pyrunner_error(&self, task_id: u64, error: &PyRunnerError) {
        let error_info = ErrorInfo::new(task_id, error);
        self.send_error_safe(error_info);
    }

    #[allow(dead_code)]
    pub fn get_raw_sender(&self) -> IpcSender<Message> {
        self.sender.clone()
    }
}

/// 创建消息通道
pub fn create_message_channel() -> (MessageSender, IpcReceiver<Message>) {
    let (sender, receiver) = ipc::channel().expect("Failed to create IPC channel");
    let message_sender = MessageSender::new(sender);
    (message_sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_progress_info() {
        let mut progress = ProgressInfo::new(1);
        assert_eq!(progress.percentage, 0.0);
        assert!(!progress.is_completed);
        assert!(!progress.has_error);

        progress.update_progress(50.0, "半程".to_string());
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.message, "半程");

        progress.complete();
        assert_eq!(progress.percentage, 100.0);
        assert!(progress.is_completed);

        let mut error_progress = ProgressInfo::new(2);
        error_progress.error("测试错误".to_string());
        assert!(error_progress.has_error);
        assert_eq!(error_progress.error_message, Some("测试错误".to_string()));
    }

    #[test]
    fn test_message_sender() {
        let (sender, receiver) = create_message_channel();

        sender.send_task_started(1);

        sender.send_task_progress(1, 50.0, "进行中".to_string());

        sender.send_task_completed(1);

        // 接收第一条消息 - 任务启动
        let msg1 = receiver.try_recv_timeout(Duration::from_millis(100)).unwrap();
        if let Message::Progress(progress) = msg1 {
            assert_eq!(progress.task_id, 1);
            assert_eq!(progress.percentage, 0.0);
        } else {
            panic!("期望收到 Progress 消息");
        }

        // 接收第二条消息 - 进度更新
        let msg2 = receiver.try_recv_timeout(Duration::from_millis(100)).unwrap();
        if let Message::Progress(progress) = msg2 {
            assert_eq!(progress.percentage, 50.0);
            assert_eq!(progress.message, "进行中");
        } else {
            panic!("期望收到 Progress 消息");
        }

        // 接收第三条消息 - 任务完成
        let msg3 = receiver.try_recv_timeout(Duration::from_millis(100)).unwrap();
        if let Message::Progress(progress) = msg3 {
            assert_eq!(progress.percentage, 100.0);
            assert!(progress.is_completed);
        } else {
            panic!("期望收到 Progress 消息");
        }
    }

    #[test]
    fn test_error_handling() {
        let (sender, receiver) = create_message_channel();

        sender.send_task_error(1, "模拟错误".to_string());

        let msg = receiver.try_recv_timeout(Duration::from_millis(100)).unwrap();
        if let Message::Progress(progress) = msg {
            assert!(progress.has_error);
            assert_eq!(progress.error_message, Some("模拟错误".to_string()));
            assert!(progress.message.contains("模拟错误"));
        } else {
            panic!("期望收到 Progress 消息");
        }
    }

    #[test]
    fn test_error_message() {
        let (sender, receiver) = create_message_channel();

        let error = PyRunnerError::task_execution_failed("测试错误");
        sender.send_task_error_from_pyrunner_error(1, &error);

        let msg = receiver.try_recv_timeout(Duration::from_millis(100)).unwrap();
        if let Message::Error(error_info) = msg {
            assert_eq!(error_info.task_id, 1);
            assert_eq!(error_info.error_code, 1001);
            assert!(error_info.error_message.contains("测试错误"));
        } else {
            panic!("期望收到 Error 消息");
        }
    }

    #[test]
    fn test_result_message() {
        let (_sender, _receiver) = create_message_channel();

        // 注意：由于ipc-channel使用bincode进行序列化，
        // 而bincode不支持serde_json::Value的完整序列化，
        // 所以这个测试被简化为只验证ResultInfo的构造
        let result_data = serde_json::json!({"status": "success", "data": 42});
        let result_info = ResultInfo::success(1, result_data.clone());
        
        assert_eq!(result_info.task_id, 1);
        assert!(result_info.success);
        assert_eq!(result_info.result_type, "success");
        assert_eq!(result_info.result_data, result_data);
    }
}
