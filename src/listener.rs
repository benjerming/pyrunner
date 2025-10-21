use tracing::{info, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt as _;

use crate::ipc::{ErrorMessage, Message, ProgressMessage, ResultMessage};

pub trait MessageListener {
    fn on_message(&mut self, message: String) {
        info!("on_message: {message}");
        if let Ok(message) = serde_json::from_str(&message) {
            match message {
                Message::Progress(progress) => self.on_progress(progress),
                Message::Error(error) => self.on_error(error),
                Message::Result(result) => self.on_result(result),
            }
        }
    }
    fn on_progress(&mut self, progress: ProgressMessage);
    fn on_error(&mut self, error: ErrorMessage);
    fn on_result(&mut self, result: ResultMessage);
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
    fn on_progress(&mut self, progress: ProgressMessage) {
        if progress.size > 0 {
            self.span.pb_set_length(progress.size);
        }
        self.span.pb_set_position(progress.done);
    }

    fn on_error(&mut self, error: ErrorMessage) {
        self.span
            .pb_set_finish_message(&format!("❌ 任务出错: {}", error.error_message));
    }

    fn on_result(&mut self, result: ResultMessage) {
        self.span.pb_set_finish_message(&format!(
            "✅ 任务完成: {} 页，{} 字",
            result.pages, result.words
        ));
    }
}
