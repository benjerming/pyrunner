use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

/// 进度信息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// 任务ID
    pub task_id: String,
    /// 当前进度百分比 (0-100)
    pub percentage: f64,
    /// 进度描述信息
    pub message: String,
    /// 当前步骤
    pub current_step: u32,
    /// 总步骤数
    pub total_steps: u32,
    /// 是否完成
    pub is_completed: bool,
    /// 是否出错
    pub has_error: bool,
    /// 错误信息
    pub error_message: Option<String>,
}

impl ProgressInfo {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            percentage: 0.0,
            message: "开始任务".to_string(),
            current_step: 0,
            total_steps: 100,
            is_completed: false,
            has_error: false,
            error_message: None,
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
    }
}

/// 消息发送器
#[derive(Clone)]
pub struct MessageSender {
    sender: mpsc::Sender<ProgressInfo>,
}

impl MessageSender {
    /// 创建新的消息发送器
    pub fn new(sender: mpsc::Sender<ProgressInfo>) -> Self {
        Self { sender }
    }

    /// 发送进度更新
    pub fn send_progress(
        &self,
        progress: ProgressInfo,
    ) -> Result<(), mpsc::SendError<ProgressInfo>> {
        debug!("发送进度更新: {:?}", progress);
        self.sender.send(progress)
    }

    /// 发送进度更新（忽略错误）
    pub fn send_progress_safe(&self, progress: ProgressInfo) {
        if let Err(e) = self.send_progress(progress) {
            error!("发送进度更新失败: {}", e);
        }
    }

    /// 发送任务开始消息
    pub fn send_task_started(&self, task_id: String) {
        let progress = ProgressInfo::new(task_id);
        self.send_progress_safe(progress);
    }

    /// 发送任务进度更新
    pub fn send_task_progress(&self, task_id: String, percentage: f64, message: String) {
        let mut progress = ProgressInfo::new(task_id);
        progress.update_progress(percentage, message);
        self.send_progress_safe(progress);
    }

    /// 发送任务完成消息
    pub fn send_task_completed(&self, task_id: String) {
        let mut progress = ProgressInfo::new(task_id);
        progress.complete();
        self.send_progress_safe(progress);
    }

    /// 发送任务错误消息
    pub fn send_task_error(&self, task_id: String, error_msg: String) {
        let mut progress = ProgressInfo::new(task_id);
        progress.error(error_msg);
        self.send_progress_safe(progress);
    }

    /// 获取底层发送器的克隆
    pub fn get_raw_sender(&self) -> mpsc::Sender<ProgressInfo> {
        self.sender.clone()
    }
}

/// 创建消息通道，返回发送器和接收器
pub fn create_message_channel() -> (MessageSender, mpsc::Receiver<ProgressInfo>) {
    let (sender, receiver) = mpsc::channel();
    let message_sender = MessageSender::new(sender);
    (message_sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_progress_info() {
        let mut progress = ProgressInfo::new("test_task".to_string());
        assert_eq!(progress.percentage, 0.0);
        assert!(!progress.is_completed);
        assert!(!progress.has_error);

        progress.update_progress(50.0, "半程".to_string());
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.message, "半程");

        progress.complete();
        assert_eq!(progress.percentage, 100.0);
        assert!(progress.is_completed);

        let mut error_progress = ProgressInfo::new("error_task".to_string());
        error_progress.error("测试错误".to_string());
        assert!(error_progress.has_error);
        assert_eq!(error_progress.error_message, Some("测试错误".to_string()));
    }

    #[test]
    fn test_message_sender() {
        let (sender, receiver) = create_message_channel();

        // 发送任务开始消息
        sender.send_task_started("test_task".to_string());

        // 发送进度更新
        sender.send_task_progress("test_task".to_string(), 50.0, "进行中".to_string());

        // 发送任务完成消息
        sender.send_task_completed("test_task".to_string());

        // 接收消息
        let msg1 = receiver.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(msg1.task_id, "test_task");
        assert_eq!(msg1.percentage, 0.0);

        let msg2 = receiver.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(msg2.percentage, 50.0);
        assert_eq!(msg2.message, "进行中");

        let msg3 = receiver.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(msg3.percentage, 100.0);
        assert!(msg3.is_completed);
    }

    #[test]
    fn test_error_handling() {
        let (sender, receiver) = create_message_channel();

        sender.send_task_error("error_task".to_string(), "模拟错误".to_string());

        let msg = receiver.recv_timeout(Duration::from_millis(100)).unwrap();
        assert!(msg.has_error);
        assert_eq!(msg.error_message, Some("模拟错误".to_string()));
        assert!(msg.message.contains("模拟错误"));
    }
}
