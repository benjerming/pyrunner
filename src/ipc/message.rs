use crate::error::PyRunnerError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    Progress(ProgressMessage),
    Error(ErrorMessage),
    Result(ResultMessage),
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressMessage {
    pub done: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorMessage {
    pub error_code: i32,
    pub error_message: String,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResultMessage {
    pub pages: u64,
    pub words: u64,
}

impl ProgressMessage {
    pub fn new(done: u64, size: u64) -> Self {
        Self { done, size }
    }
}

impl From<&PyRunnerError> for ErrorMessage {
    fn from(error: &PyRunnerError) -> Self {
        Self {
            error_code: error.error_code(),
            error_message: error.to_string(),
        }
    }
}

impl ErrorMessage {
    pub fn new(error_code: i32, error_message: String) -> Self {
        Self {
            error_code,
            error_message,
        }
    }
}

impl ResultMessage {
    pub fn new(pages: u64, words: u64) -> Self {
        Self { pages, words }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message() {
        let message = Message::Error(ErrorMessage::new(1001, "测试错误".into()));
        let serialized = serde_json::to_string(&message).unwrap();
        println!("Message(Error(ErrorMessage)) serialized: {serialized}");
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, message);

        let message = Message::Progress(ProgressMessage::new(0, 100));
        let serialized = serde_json::to_string(&message).unwrap();
        println!("Message(Progress(ProgressMessage)) serialized: {serialized}");
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, message);

        let message = Message::Result(ResultMessage::new(10, 5000));
        let serialized = serde_json::to_string(&message).unwrap();
        println!("Message(Result(ResultMessage)) serialized: {serialized}");
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_progress_info() {
        let progress = ProgressMessage::new(0, 100);
        assert_eq!(progress.done, 0);
        assert_eq!(progress.size, 100);

        let serialized = serde_json::to_string(&progress).unwrap();
        println!("ProgressMessage serialized: {serialized}");
        let deserialized: ProgressMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.done, 0);
        assert_eq!(deserialized.size, 100);
    }

    #[test]
    fn test_error_info() {
        let error = PyRunnerError::task_execution_failed("测试错误");
        let code = error.error_code();
        let message = error.to_string();

        let error_info = ErrorMessage::from(&error);
        assert_eq!(error_info.error_code, code);
        assert_eq!(error_info.error_message, message);

        let serialized = serde_json::to_string(&error_info).unwrap();
        println!("ErrorMessage serialized: {serialized}");
        let deserialized: ErrorMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.error_code, code);
        assert_eq!(deserialized.error_message, message);
    }

    #[test]
    fn test_result_info() {
        let result_info = ResultMessage::new(10, 5000);
        assert_eq!(result_info.pages, 10);
        assert_eq!(result_info.words, 5000);

        let serialized = serde_json::to_string(&result_info).unwrap();
        println!("ResultMessage serialized: {serialized}");
        let deserialized: ResultMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.pages, 10);
        assert_eq!(deserialized.words, 5000);
    }
}
