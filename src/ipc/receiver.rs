use super::message::{ErrorMessage, Message, ProgressMessage, ResultMessage};
use indicatif::{ProgressBar, ProgressStyle};
use ipc_channel::ipc::IpcReceiver;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{Span, debug, error, info};
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub trait MessageListener: Send + Sync {
    fn on_progress_update(&self, progress: &ProgressMessage);
    fn on_error(&self, error: &ErrorMessage);
    fn on_result(&self, result: &ResultMessage);
}

pub struct ConsoleMessageListener;

impl MessageListener for ConsoleMessageListener {
    fn on_progress_update(&self, progress: &ProgressMessage) {
        let percentage = if progress.size > 0 {
            (progress.done as f64 / progress.size as f64) * 100.0
        } else {
            0.0
        };

        let bar_length = 50;
        let filled_length = (percentage / 100.0 * bar_length as f64) as usize;
        let bar = "█".repeat(filled_length) + &"░".repeat(bar_length - filled_length);

        info!(
            "\r[{}] {:.1}% - 任务 {} ({}/{})",
            bar, percentage, progress.task_id, progress.done, progress.size
        );
    }

    fn on_error(&self, error: &ErrorMessage) {
        error!("\n❌ 错误消息:");
        error!("  任务ID: {}", error.task_id);
        error!("  错误码: {}", error.error_code);
        error!("  错误信息: {}", error.error_message);
    }

    fn on_result(&self, result: &ResultMessage) {
        info!("\n✅ 执行结果:");
        info!("  任务ID: {}", result.task_id);
        info!("  页数: {}", result.pages);
        info!("  字数: {}", result.words);
    }
}

pub struct ConsoleProgressListener {
    span: Span,
    task_id: u64,
}

impl ConsoleProgressListener {
    pub fn new(task_id: u64, span: Span) -> Self {
        span.pb_set_message(&format!("task_id: {task_id}"));
        Self { span, task_id }
    }
}

impl MessageListener for ConsoleProgressListener {
    fn on_progress_update(&self, progress: &ProgressMessage) {
        if progress.size > 0 {
            self.span.pb_set_length(progress.size);
        }
        self.span.pb_set_position(progress.done);
        self.span
            .pb_set_message(&format!("task_id: {}", progress.task_id));
    }

    fn on_error(&self, error: &ErrorMessage) {
        self.span
            .pb_set_finish_message(&format!("❌ 任务出错: {}", error.error_message));
    }

    fn on_result(&self, result: &ResultMessage) {
        self.span.pb_set_finish_message(&format!(
            "✅ 任务完成: {} 页，{} 字",
            result.pages, result.words
        ));
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
                    info!("{message:?}");

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
                            break;
                        }
                        Message::Result(result) => {
                            for listener in &self.listeners {
                                listener.on_result(result);
                            }
                            break;
                        }
                    }
                }
                Err(ipc_channel::ipc::TryRecvError::Empty) => {
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
        fn on_progress_update(&self, progress: &ProgressMessage) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("progress: {}/{}", progress.done, progress.size));
        }

        fn on_error(&self, error: &ErrorMessage) {
            let mut messages = self.messages.lock().unwrap();
            messages.push(format!("error: {}", error.task_id));
        }

        fn on_result(&self, result: &ResultMessage) {
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
