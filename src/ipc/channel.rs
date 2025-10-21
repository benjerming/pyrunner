use std::sync::{Arc, Mutex};

use super::receiver::MessageListener;
use super::receiver::MessageReceiver;
use super::sender::MessageSender;
use ipc_channel::ipc;

pub fn create_message_channel(
    listener: Arc<Mutex<dyn MessageListener>>,
) -> (MessageSender, MessageReceiver) {
    let (sender, receiver) = ipc::channel().expect("Failed to create IPC channel");
    let message_sender = MessageSender::new(sender);
    let message_receiver = MessageReceiver::new(receiver).with_listener(listener);
    (message_sender, message_receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::message::{ErrorMessage, ProgressMessage, ResultMessage};

    #[test]
    fn test_create_channel() {
        #[derive(Default)]
        struct TestProgressListener {
            progress_count: u32,
            error_count: u32,
            result_count: u32,
        }
        impl MessageListener for TestProgressListener {
            fn on_progress(&mut self, progress: &ProgressMessage) {
                self.progress_count += 1;
                println!("on_progress_update: {progress:?}");
            }
            fn on_error(&mut self, error: &ErrorMessage) {
                self.error_count += 1;
                println!("on_error: {error:?}");
            }
            fn on_result(&mut self, result: &ResultMessage) {
                self.result_count += 1;
                println!("on_result: {result:?}");
            }
        }
        let test_listener = Arc::new(Mutex::new(TestProgressListener::default()));
        let (sender, receiver) = create_message_channel(test_listener.clone());

        sender.send_progress_safe(ProgressMessage::new(1));
        sender.send_error_safe(ErrorMessage::from_string(1, "test error".to_string()));
        sender.send_result_safe(ResultMessage::new(1, 100, 100));

        drop(sender);
        receiver.start_listening();
        let guard = test_listener.lock().unwrap();
        assert_eq!(guard.progress_count, 1);
        assert_eq!(guard.error_count, 1);
        assert_eq!(guard.result_count, 1);
    }
}
