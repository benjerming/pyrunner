use serde::{Deserialize, Serialize};
use crate::error::PyRunnerError;

/// 统一的消息枚举，支持多种消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// 进度消息
    Progress(ProgressInfo),
    /// 错误消息
    Error(ErrorInfo),
    /// 结果消息
    Result(ResultInfo),
}

/// 进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub task_id: u64,
    pub done: u64,
    pub size: u64,
}

/// 错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub task_id: u64,
    pub error_code: i32,
    pub error_message: String,
}

/// 执行结果信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultInfo {
    pub task_id: u64,
    pub pages: u64,
    pub words: u64,
}

impl ProgressInfo {
    pub fn new(task_id: u64) -> Self {
        Self {
            task_id,
            done: 0,
            size: 0,
        }
    }

    pub fn update_progress(&mut self, done: u64, size: u64) {
        self.done = done;
        self.size = size;
    }
}

impl ErrorInfo {
    pub fn new(task_id: u64, error: &PyRunnerError) -> Self {
        Self {
            task_id,
            error_code: error.error_code(),
            error_message: error.to_string(),
        }
    }

    pub fn from_string(task_id: u64, error_message: String) -> Self {
        Self {
            task_id,
            error_code: 9999,
            error_message,
        }
    }
}

impl ResultInfo {
    pub fn new(task_id: u64, pages: u64, words: u64) -> Self {
        Self {
            task_id,
            pages,
            words,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_info() {
        let mut progress = ProgressInfo::new(1);
        assert_eq!(progress.task_id, 1);
        assert_eq!(progress.done, 0);
        assert_eq!(progress.size, 0);

        progress.update_progress(50, 100);
        assert_eq!(progress.done, 50);
        assert_eq!(progress.size, 100);
    }

    #[test]
    fn test_error_info() {
        let error = PyRunnerError::task_execution_failed("测试错误");
        let error_info = ErrorInfo::new(1, &error);
        assert_eq!(error_info.task_id, 1);
        assert_eq!(error_info.error_code, 1001);
        assert!(error_info.error_message.contains("测试错误"));
    }

    #[test]
    fn test_result_info() {
        let result_info = ResultInfo::new(1, 10, 5000);
        
        assert_eq!(result_info.task_id, 1);
        assert_eq!(result_info.pages, 10);
        assert_eq!(result_info.words, 5000);
    }
}

