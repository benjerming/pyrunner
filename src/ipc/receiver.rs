#[allow(unused_imports)]
use tracing::{debug, error, info};
use ipc_channel::ipc::IpcReceiver;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use indicatif::{ProgressBar, ProgressStyle};
use super::message::{Message, ProgressInfo, ErrorInfo, ResultInfo};

/// 消息监听器trait - 处理各种类型的消息
pub trait MessageListener: Send + Sync {
    /// 进度更新回调
    fn on_progress_update(&self, progress: &ProgressInfo);

    /// 错误消息回调
    fn on_error(&self, error: &ErrorInfo);

    /// 结果消息回调
    fn on_result(&self, result: &ResultInfo);
}

/// 控制台消息监听器 - 完整实现MessageListener
pub struct ConsoleMessageListener;

impl MessageListener for ConsoleMessageListener {
    fn on_progress_update(&self, progress: &ProgressInfo) {
        let percentage = if progress.size > 0 {
            (progress.done as f64 / progress.size as f64) * 100.0
        } else {
            0.0
        };

        let bar_length = 50;
        let filled_length = (percentage / 100.0 * bar_length as f64) as usize;
        let bar = "█".repeat(filled_length) + &"░".repeat(bar_length - filled_length);

        println!(
            "\r[{}] {:.1}% - 任务 {} ({}/{})",
            bar, percentage, progress.task_id, progress.done, progress.size
        );
    }

    fn on_error(&self, error: &ErrorInfo) {
        println!("\n❌ 错误消息:");
        println!("  任务ID: {}", error.task_id);
        println!("  错误码: {}", error.error_code);
        println!("  错误信息: {}", error.error_message);
    }

    fn on_result(&self, result: &ResultInfo) {
        println!("\n✅ 执行结果:");
        println!("  任务ID: {}", result.task_id);
        println!("  页数: {}", result.pages);
        println!("  字数: {}", result.words);
    }
}

/// 使用 indicatif 实现的进度监听器
pub struct ConsoleProgressListener {
    progress_bars: Arc<Mutex<HashMap<String, ProgressBar>>>,
}

impl ConsoleProgressListener {
    pub fn new() -> Self {
        Self {
            progress_bars: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_or_create_progress_bar(&self, task_id: &str, total: u64) -> ProgressBar {
        let mut bars = self.progress_bars.lock().unwrap();
        
        if let Some(bar) = bars.get(task_id) {
            bar.clone()
        } else {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                    .unwrap()
                    .progress_chars("█▓▒░ ")
            );
            bars.insert(task_id.to_string(), pb.clone());
            pb
        }
    }

    fn remove_progress_bar(&self, task_id: &str) {
        let mut bars = self.progress_bars.lock().unwrap();
        if let Some(pb) = bars.remove(task_id) {
            pb.finish_and_clear();
        }
    }
}

impl Default for ConsoleProgressListener {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageListener for ConsoleProgressListener {
    fn on_progress_update(&self, progress: &ProgressInfo) {
        let task_id_str = progress.task_id.to_string();
        let pb = self.get_or_create_progress_bar(
            &task_id_str,
            progress.size
        );
        
        pb.set_position(progress.done);
        pb.set_message(format!("任务 {}", progress.task_id));
    }

    fn on_error(&self, error: &ErrorInfo) {
        let task_id_str = error.task_id.to_string();
        
        if let Some(pb) = self.progress_bars.lock().unwrap().get(&task_id_str) {
            pb.finish_with_message(format!("❌ 任务出错: {}", error.error_message));
        }
        
        self.remove_progress_bar(&task_id_str);
    }

    fn on_result(&self, result: &ResultInfo) {
        let task_id_str = result.task_id.to_string();
        
        if let Some(pb) = self.progress_bars.lock().unwrap().get(&task_id_str) {
            pb.finish_with_message(format!("✅ 任务完成: {} 页，{} 字", result.pages, result.words));
        }
        
        self.remove_progress_bar(&task_id_str);
    }
}

pub struct MessageReceiver {
    receiver: IpcReceiver<Message>,
    listeners: Vec<Box<dyn MessageListener>>,
    timeout: Duration,
}

impl MessageReceiver {
    pub fn new(receiver: IpcReceiver<Message>) -> Self {
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
            match self.receiver.try_recv_timeout(self.timeout) {
                Ok(message) => {
                    info!("收到消息: {:?}", message);

                    match &message {
                        Message::Progress(progress) => {
                            for listener in &self.listeners {
                                listener.on_progress_update(progress);
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
                Err(ipc_channel::ipc::TryRecvError::Empty) => {
                    // 超时，继续等待
                    continue;
                }
                Err(ipc_channel::ipc::TryRecvError::IpcError(_)) => {
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
    use crate::ipc::create_message_channel;
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

    impl MessageListener for TestProgressListener {
        fn on_progress_update(&self, progress: &ProgressInfo) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("progress: {}/{}", progress.done, progress.size));
        }

        fn on_error(&self, error: &ErrorInfo) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("error: {}", error.task_id));
        }

        fn on_result(&self, result: &ResultInfo) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("result: {}", result.task_id));
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
            sender_clone.send_task_started(1);
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_progress(1, 50, 100);
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_completed(1);
        });

        message_receiver.start_listening();

        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), 3);
        assert!(messages[0].contains("progress: 0/0"));
        assert!(messages[1].contains("progress: 50/100"));
        assert!(messages[2].contains("result: 1"));
    }
}

