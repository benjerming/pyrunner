use crate::message_sender::{Message, ProgressInfo, ErrorInfo, ResultInfo};
#[allow(unused_imports)]
use log::{debug, error, info};
use std::sync::mpsc;
use std::time::Duration;

/// 消息监听器trait - 处理各种类型的消息
pub trait MessageListener: Send + Sync {
    /// 进度更新回调
    fn on_progress_update(&self, progress: &ProgressInfo);

    /// 任务完成回调
    fn on_task_completed(&self, progress: &ProgressInfo);

    /// 任务出错回调（通过ProgressInfo）
    fn on_task_error(&self, progress: &ProgressInfo);

    /// 错误消息回调
    fn on_error(&self, error: &ErrorInfo);

    /// 结果消息回调
    fn on_result(&self, result: &ResultInfo);
}

/// 保留旧的ProgressListener trait以保持向后兼容
pub trait ProgressListener: Send + Sync {
    fn on_progress_update(&self, progress: &ProgressInfo);

    fn on_task_completed(&self, progress: &ProgressInfo);

    fn on_task_error(&self, progress: &ProgressInfo);
}

/// 为实现了ProgressListener的类型自动实现MessageListener
impl<T: ProgressListener> MessageListener for T {
    fn on_progress_update(&self, progress: &ProgressInfo) {
        ProgressListener::on_progress_update(self, progress);
    }

    fn on_task_completed(&self, progress: &ProgressInfo) {
        ProgressListener::on_task_completed(self, progress);
    }

    fn on_task_error(&self, progress: &ProgressInfo) {
        ProgressListener::on_task_error(self, progress);
    }

    fn on_error(&self, error: &ErrorInfo) {
        // 默认实现：将ErrorInfo转换为ProgressInfo并调用on_task_error
        let mut progress = ProgressInfo::new(error.task_id.clone());
        progress.error(error.error_message.clone());
        self.on_task_error(&progress);
    }

    fn on_result(&self, _result: &ResultInfo) {
        // 默认实现：不处理结果消息
    }
}

/// 控制台消息监听器 - 完整实现MessageListener
pub struct ConsoleMessageListener;

impl MessageListener for ConsoleMessageListener {
    fn on_progress_update(&self, progress: &ProgressInfo) {
        let bar_length = 50;
        let filled_length = (progress.percentage / 100.0 * bar_length as f64) as usize;
        let bar = "█".repeat(filled_length) + &"░".repeat(bar_length - filled_length);

        println!(
            "\r[{}] {:.1}% - {} ({}/{})",
            bar, progress.percentage, progress.message, progress.current_step, progress.total_steps
        );
    }

    fn on_task_completed(&self, progress: &ProgressInfo) {
        println!("\n✅ 任务完成: {} - {}", progress.task_id, progress.message);
    }

    fn on_task_error(&self, progress: &ProgressInfo) {
        println!("\n❌ 任务出错: {} - {}", progress.task_id, progress.message);
        if let Some(error) = &progress.error_message {
            println!("错误详情: {}", error);
        }
    }

    fn on_error(&self, error: &ErrorInfo) {
        println!("\n❌ 错误消息:");
        println!("  任务ID: {}", error.task_id);
        println!("  错误码: {}", error.error_code);
        println!("  错误信息: {}", error.error_message);
        println!("  可重试: {}", if error.is_retryable { "是" } else { "否" });
        println!("  致命错误: {}", if error.is_fatal { "是" } else { "否" });
    }

    fn on_result(&self, result: &ResultInfo) {
        let status = if result.success { "✅ 成功" } else { "❌ 失败" };
        println!("\n{} 执行结果:", status);
        println!("  任务ID: {}", result.task_id);
        println!("  结果类型: {}", result.result_type);
        println!("  结果数据: {}", result.result_data);
    }
}

/// 保留旧的ConsoleProgressListener以保持向后兼容
pub struct ConsoleProgressListener;

impl ProgressListener for ConsoleProgressListener {
    fn on_progress_update(&self, progress: &ProgressInfo) {
        let bar_length = 50;
        let filled_length = (progress.percentage / 100.0 * bar_length as f64) as usize;
        let bar = "█".repeat(filled_length) + &"░".repeat(bar_length - filled_length);

        println!(
            "\r[{}] {:.1}% - {} ({}/{})",
            bar, progress.percentage, progress.message, progress.current_step, progress.total_steps
        );
    }

    fn on_task_completed(&self, progress: &ProgressInfo) {
        println!("\n✅ 任务完成: {} - {}", progress.task_id, progress.message);
    }

    fn on_task_error(&self, progress: &ProgressInfo) {
        println!("\n❌ 任务出错: {} - {}", progress.task_id, progress.message);
        if let Some(error) = &progress.error_message {
            println!("错误详情: {}", error);
        }
    }
}

pub struct MessageReceiver {
    receiver: mpsc::Receiver<Message>,
    listeners: Vec<Box<dyn MessageListener>>,
    timeout: Duration,
}

impl MessageReceiver {
    pub fn new(receiver: mpsc::Receiver<Message>) -> Self {
        Self {
            receiver,
            listeners: Vec::new(),
            timeout: Duration::from_millis(100),
        }
    }

    pub fn add_listener(&mut self, listener: Box<dyn MessageListener>) {
        self.listeners.push(listener);
    }

    pub fn start_listening(&self) {
        info!("开始监听消息...");

        loop {
            match self.receiver.recv_timeout(self.timeout) {
                Ok(message) => {
                    debug!("收到消息: {:?}", message);

                    match &message {
                        Message::Progress(progress) => {
                            for listener in &self.listeners {
                                if progress.has_error {
                                    listener.on_task_error(progress);
                                } else if progress.is_completed {
                                    listener.on_task_completed(progress);
                                } else {
                                    listener.on_progress_update(progress);
                                }
                            }

                            // 如果任务完成或出错，停止监听
                            if progress.is_completed || progress.has_error {
                                break;
                            }
                        }
                        Message::Error(error) => {
                            for listener in &self.listeners {
                                listener.on_error(error);
                            }
                            // 收到错误消息后停止监听
                            break;
                        }
                        Message::Result(result) => {
                            for listener in &self.listeners {
                                listener.on_result(result);
                            }
                            // 收到结果消息后停止监听
                            break;
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    info!("发送器已断开连接，停止监听");
                    break;
                }
            }
        }

        info!("监听结束");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_sender::create_message_channel;
    use std::sync::{Arc, Mutex};
    use std::thread;

    struct TestProgressListener {
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl TestProgressListener {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let messages = Arc::new(Mutex::new(Vec::new()));
            let listener = Self {
                messages: messages.clone(),
            };
            (listener, messages)
        }
    }

    impl ProgressListener for TestProgressListener {
        fn on_progress_update(&self, progress: &ProgressInfo) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("progress: {}%", progress.percentage));
        }

        fn on_task_completed(&self, progress: &ProgressInfo) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("completed: {}", progress.task_id));
        }

        fn on_task_error(&self, progress: &ProgressInfo) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("error: {}", progress.task_id));
        }
    }

    #[test]
    fn test_message_receiver() {
        let (sender, receiver) = create_message_channel();
        let mut message_receiver = MessageReceiver::new(receiver);

        let (test_listener, messages) = TestProgressListener::new();
        message_receiver.add_listener(Box::new(test_listener));

        let sender_clone = sender.clone();
        thread::spawn(move || {
            sender_clone.send_task_started("test_task".to_string());
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_progress("test_task".to_string(), 50.0, "进行中".to_string());
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_completed("test_task".to_string());
        });

        message_receiver.start_listening();

        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), 3);
        assert!(messages[0].contains("progress: 0"));
        assert!(messages[1].contains("progress: 50"));
        assert!(messages[2].contains("completed: test_task"));
    }
}
