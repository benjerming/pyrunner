use super::message::Message;
use crate::listener::MessageListener;
use ipc_channel::ipc::{IpcError, IpcReceiver, TryRecvError};
use std::sync::Mutex;
use std::{sync::Arc, time::Duration};
use tracing::{error, info};
use tracing::{instrument};

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

                    match message {
                        Message::Progress(progress) => {
                            for listener in &self.listeners {
                                if let Ok(mut l) = listener.lock() {
                                    l.on_progress(progress);
                                }
                            }
                        }
                        Message::Error(e) => {
                            for listener in &self.listeners {
                                if let Ok(mut l) = listener.lock() {
                                    l.on_error(e.clone());
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
