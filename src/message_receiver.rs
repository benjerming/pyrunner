use crate::message_sender::ProgressInfo;
use log::{debug, error, info};
use std::sync::mpsc;
use std::time::Duration;

/// 进度监听器trait
pub trait ProgressListener: Send + Sync {
    /// 当进度更新时调用
    fn on_progress_update(&self, progress: &ProgressInfo);

    /// 当任务完成时调用
    fn on_task_completed(&self, progress: &ProgressInfo);

    /// 当任务出错时调用
    fn on_task_error(&self, progress: &ProgressInfo);
}

/// 控制台进度监听器实现
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

/// 消息接收器
pub struct MessageReceiver {
    receiver: mpsc::Receiver<ProgressInfo>,
    listeners: Vec<Box<dyn ProgressListener>>,
    timeout: Duration,
}

impl MessageReceiver {
    /// 创建新的消息接收器
    pub fn new(receiver: mpsc::Receiver<ProgressInfo>) -> Self {
        Self {
            receiver,
            listeners: Vec::new(),
            timeout: Duration::from_millis(100),
        }
    }

    /// 添加进度监听器
    pub fn add_listener(&mut self, listener: Box<dyn ProgressListener>) {
        self.listeners.push(listener);
    }

    /// 启动监听循环（阻塞）
    pub fn start_listening(&self) {
        info!("开始监听进度更新...");

        loop {
            match self.receiver.recv_timeout(self.timeout) {
                Ok(progress) => {
                    debug!("收到进度更新: {:?}", progress);

                    // 通知所有监听器
                    for listener in &self.listeners {
                        if progress.has_error {
                            listener.on_task_error(&progress);
                        } else if progress.is_completed {
                            listener.on_task_completed(&progress);
                        } else {
                            listener.on_progress_update(&progress);
                        }
                    }

                    // 如果任务完成或出错，退出监听循环
                    if progress.is_completed || progress.has_error {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 超时，继续等待
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    info!("消息发送器已断开连接，停止监听");
                    break;
                }
            }
        }

        info!("消息接收器监听结束");
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_sender::{MessageSender, create_message_channel};
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// 测试用的监听器，记录收到的消息
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

        // 在另一个线程中发送消息
        let sender_clone = sender.clone();
        thread::spawn(move || {
            sender_clone.send_task_started("test_task".to_string());
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_progress("test_task".to_string(), 50.0, "进行中".to_string());
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_completed("test_task".to_string());
        });

        // 处理消息
        message_receiver.start_listening();

        // 检查收到的消息
        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), 3);
        assert!(messages[0].contains("progress: 0"));
        assert!(messages[1].contains("progress: 50"));
        assert!(messages[2].contains("completed: test_task"));
    }
}
