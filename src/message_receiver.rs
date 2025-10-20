use crate::message_sender::{Message, ProgressInfo, ErrorInfo, ResultInfo};
#[allow(unused_imports)]
use tracing::{debug, error, info};
use ipc_channel::ipc::IpcReceiver;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use indicatif::{ProgressBar, ProgressStyle};

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
/// 使用 tracing-indicatif 实现的进度监听器
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

impl ProgressListener for ConsoleProgressListener {
    fn on_progress_update(&self, progress: &ProgressInfo) {
        let task_id_str = progress.task_id.to_string();
        let pb = self.get_or_create_progress_bar(
            &task_id_str,
            progress.total_steps as u64
        );
        
        pb.set_position(progress.current_step as u64);
        pb.set_message(progress.message.clone());
    }

    fn on_task_completed(&self, progress: &ProgressInfo) {
        let task_id_str = progress.task_id.to_string();
        
        if let Some(pb) = self.progress_bars.lock().unwrap().get(&task_id_str) {
            pb.finish_with_message(format!("✅ 任务完成: {}", progress.message));
        }
        
        // 稍后清除进度条以确保完成消息可见
        self.remove_progress_bar(&task_id_str);
    }

    fn on_task_error(&self, progress: &ProgressInfo) {
        let task_id_str = progress.task_id.to_string();
        
        if let Some(pb) = self.progress_bars.lock().unwrap().get(&task_id_str) {
            let error_msg = if let Some(error) = &progress.error_message {
                format!("❌ 任务出错: {} - 错误详情: {}", progress.message, error)
            } else {
                format!("❌ 任务出错: {}", progress.message)
            };
            pb.finish_with_message(error_msg);
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
            sender_clone.send_task_started(1);
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_progress(1, 50.0, "进行中".to_string());
            thread::sleep(Duration::from_millis(10));
            sender_clone.send_task_completed(1);
        });

        message_receiver.start_listening();

        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), 3);
        assert!(messages[0].contains("progress: 0"));
        assert!(messages[1].contains("progress: 50"));
        assert!(messages[2].contains("completed: 1"));
    }
}
