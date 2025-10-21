use super::message::{ErrorMessage, Message, ProgressMessage, ResultMessage};
use ipc_channel::ipc::{IpcError, IpcReceiver, TryRecvError};
use std::sync::Mutex;
use std::{sync::Arc, time::Duration};
use tracing::{info_span, instrument, trace};
#[allow(unused_imports)]
use tracing::{Span, debug, error, info};
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub trait MessageListener: Send + Sync {
    fn on_progress(&mut self, progress: &ProgressMessage);
    fn on_error(&mut self, error: &ErrorMessage);
    fn on_result(&mut self, result: &ResultMessage);
}

pub struct ConsoleProgressListener {
    span: Span,
}

impl ConsoleProgressListener {
    pub fn new(task_id: u64, span: Span) -> Self {
        span.pb_set_message(&format!("task_id: {task_id}"));
        Self { span }
    }
}

impl MessageListener for ConsoleProgressListener {
    fn on_progress(&mut self, progress: &ProgressMessage) {
        if progress.size > 0 {
            self.span.pb_set_length(progress.size);
        }
        self.span.pb_set_position(progress.done);
        self.span
            .pb_set_message(&format!("task_id: {}", progress.task_id));
    }

    fn on_error(&mut self, error: &ErrorMessage) {
        self.span
            .pb_set_finish_message(&format!("❌ 任务出错: {}", error.error_message));
    }

    fn on_result(&mut self, result: &ResultMessage) {
        self.span.pb_set_finish_message(&format!(
            "✅ 任务完成: {} 页，{} 字",
            result.pages, result.words
        ));
    }
}

pub struct MessageReceiver {
    receiver: IpcReceiver<Message>,
    listeners: Vec<Arc<Mutex<dyn MessageListener>>>,
    timeout: Duration,
}

#[allow(dead_code)]
impl MessageReceiver {
    pub fn new(receiver: IpcReceiver<Message>) -> Self {
        Self {
            receiver,
            listeners: Vec::new(),
            timeout: Duration::from_millis(100),
        }
    }

    pub fn add_listener(&mut self, listener: Arc<Mutex<dyn MessageListener>>) {
        self.listeners.push(listener);
    }

    pub fn with_listener(mut self, listener: Arc<Mutex<dyn MessageListener>>) -> Self {
        self.add_listener(listener);
        self
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn recv(&self) -> Result<Message, IpcError> {
        self.receiver.recv()
    }

    pub fn try_recv(&self) -> Result<Message, TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn try_recv_timeout(&self, timeout: Duration) -> Result<Message, TryRecvError> {
        self.receiver.try_recv_timeout(timeout)
    }

    #[instrument(name = "receiver", skip(self))]
    pub fn start_listening(&self) {
        info!("开始监听消息...");

        loop {
            match self.try_recv_timeout(self.timeout) {
                Ok(message) => {
                    info!("{message:?}");

                    match &message {
                        Message::Progress(progress) => {
                            for listener in &self.listeners {
                                if let Ok(mut l) = listener.lock() {
                                    l.on_progress(progress);
                                }
                            }
                        }
                        Message::Error(error) => {
                            for listener in &self.listeners {
                                if let Ok(mut l) = listener.lock() {
                                    l.on_error(error);
                                }
                            }
                        }
                        Message::Result(result) => {
                            for listener in &self.listeners {
                                if let Ok(mut l) = listener.lock() {
                                    l.on_result(result);
                                }
                            }
                        }
                    }
                }
                Err(TryRecvError::Empty) => {
                    // trace!("监听超时，继续监听...");
                    continue;
                }
                Err(TryRecvError::IpcError(IpcError::Disconnected)) => {
                    info!("发送器已关闭连接，正常退出");
                    break;
                }
                Err(TryRecvError::IpcError(e)) => {
                    error!("监听错误: {e:?}");
                    break;
                }
            }
        }

        info!("监听结束");
    }
}
