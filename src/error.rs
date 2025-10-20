use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PyRunnerError {
    #[error("任务执行失败: {message}")]
    TaskExecutionFailed { message: String },

    #[error("任务超时: {task_id}")]
    TaskTimeout { task_id: u64 },

    #[error("任务被取消: {task_id}")]
    TaskCancelled { task_id: u64 },

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Python执行错误: {0}")]
    PythonError(String),

    #[error("Python变量未找到: {variable}")]
    PythonVariableNotFound { variable: String },

    #[error("Python模块导入失败: {module}")]
    PythonModuleImportFailed { module: String },

    #[error(transparent)]
    JniError(#[from] jni::errors::Error),

    #[error("JNI字符串转换失败")]
    JniStringConversionFailed,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("文件不存在: {path}")]
    FileNotFound { path: String },

    #[error("权限不足: {path}")]
    PermissionDenied { path: String },

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error("进程创建失败: {0}")]
    ProcessCreationFailed(String),

    #[error("进程执行失败: 退出码 {exit_code}")]
    ProcessExecutionFailed { exit_code: i32 },

    #[cfg(unix)]
    #[error(transparent)]
    NixError(#[from] nix::Error),

    #[error(transparent)]
    EnvVarError(#[from] std::env::VarError),

    #[error("消息发送失败: {0}")]
    MessageSendError(String),

    #[error("消息接收失败: {0}")]
    MessageReceiveError(String),

    #[error("通道已关闭")]
    ChannelClosed,

    #[error("配置错误: {message}")]
    ConfigError { message: String },

    #[error("参数无效: {parameter} = {value}")]
    InvalidParameter { parameter: String, value: String },

    #[error("内部错误: {message}")]
    InternalError { message: String },

    #[error("不支持的操作: {operation}")]
    UnsupportedOperation { operation: String },

    #[error("资源不足: {resource}")]
    ResourceExhausted { resource: String },

    #[error("超时: {operation}")]
    Timeout { operation: String },

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, PyRunnerError>;

impl PyRunnerError {
    pub fn task_execution_failed<S: Into<String>>(message: S) -> Self {
        Self::TaskExecutionFailed {
            message: message.into(),
        }
    }

    pub fn task_timeout(task_id: u64) -> Self {
        Self::TaskTimeout { task_id }
    }

    pub fn python_error<S: Into<String>>(message: S) -> Self {
        Self::PythonError(message.into())
    }

    pub fn python_variable_not_found<S: Into<String>>(variable: S) -> Self {
        Self::PythonVariableNotFound {
            variable: variable.into(),
        }
    }

    pub fn file_not_found<S: Into<String>>(path: S) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    pub fn permission_denied<S: Into<String>>(path: S) -> Self {
        Self::PermissionDenied { path: path.into() }
    }

    pub fn config_error<S: Into<String>>(message: S) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    pub fn internal_error<S: Into<String>>(message: S) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::TaskTimeout { .. }
                | Self::IoError(_)
                | Self::MessageSendError(_)
                | Self::MessageReceiveError(_)
                | Self::ResourceExhausted { .. }
                | Self::Timeout { .. }
        )
    }

    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::TaskCancelled { .. }
                | Self::PermissionDenied { .. }
                | Self::UnsupportedOperation { .. }
                | Self::ConfigError { .. }
        )
    }

    pub fn error_code(&self) -> i32 {
        match self {
            Self::TaskExecutionFailed { .. } => 1001,
            Self::TaskTimeout { .. } => 1002,
            Self::TaskCancelled { .. } => 1003,
            Self::JoinError(_) => 1004,
            Self::PythonError(_) => 2001,
            Self::PythonVariableNotFound { .. } => 2002,
            Self::PythonModuleImportFailed { .. } => 2003,
            Self::JniError(_) => 3001,
            Self::JniStringConversionFailed => 3002,
            Self::IoError(_) => 4001,
            Self::FileNotFound { .. } => 4002,
            Self::PermissionDenied { .. } => 4003,
            Self::JsonError(_) => 5001,
            Self::ProcessCreationFailed(_) => 6001,
            Self::ProcessExecutionFailed { .. } => 6002,
            #[cfg(unix)]
            Self::NixError(_) => 7001,
            Self::EnvVarError(_) => 7002,
            Self::MessageSendError(_) => 8001,
            Self::MessageReceiveError(_) => 8002,
            Self::ChannelClosed => 8003,
            Self::ConfigError { .. } => 9001,
            Self::InvalidParameter { .. } => 9002,
            Self::InternalError { .. } => 9999,
            Self::UnsupportedOperation { .. } => 9003,
            Self::ResourceExhausted { .. } => 9004,
            Self::Timeout { .. } => 9005,
            Self::Other(_) => 9998,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: operation.into(),
            file: None,
            line: None,
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn with_file<S: Into<String>>(mut self, file: S) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_info<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "操作: {}", self.operation)?;

        if let Some(file) = &self.file {
            write!(f, ", 文件: {}", file)?;
        }

        if let Some(line) = self.line {
            write!(f, ", 行号: {}", line)?;
        }

        if !self.additional_info.is_empty() {
            write!(f, ", 附加信息: {:?}", self.additional_info)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ContextualError {
    pub error: PyRunnerError,
    pub context: ErrorContext,
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.error, self.context)
    }
}

impl std::error::Error for ContextualError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

pub trait ResultExt<T> {
    fn with_context<F>(self, f: F) -> std::result::Result<T, ContextualError>
    where
        F: FnOnce() -> ErrorContext;

    fn with_operation<S: Into<String>>(
        self,
        operation: S,
    ) -> std::result::Result<T, ContextualError>;
}

impl<T> ResultExt<T> for Result<T> {
    fn with_context<F>(self, f: F) -> std::result::Result<T, ContextualError>
    where
        F: FnOnce() -> ErrorContext,
    {
        self.map_err(|error| ContextualError {
            error,
            context: f(),
        })
    }

    fn with_operation<S: Into<String>>(
        self,
        operation: S,
    ) -> std::result::Result<T, ContextualError> {
        self.with_context(|| ErrorContext::new(operation))
    }
}

#[macro_export]
macro_rules! context_error {
    ($error:expr, $operation:expr) => {
        $crate::error::ContextualError {
            error: $error,
            context: $crate::error::ErrorContext::new($operation),
        }
    };
    ($error:expr, $operation:expr, $($key:expr => $value:expr),+) => {
        $crate::error::ContextualError {
            error: $error,
            context: {
                let mut ctx = $crate::error::ErrorContext::new($operation);
                $(
                    ctx = ctx.with_info($key, $value);
                )+
                ctx
            },
        }
    };
}

#[macro_export]
macro_rules! error_context {
    ($operation:expr) => {
        $crate::error::ErrorContext::new($operation)
            .with_file(file!())
            .with_line(line!())
    };
    ($operation:expr, $($key:expr => $value:expr),+) => {
        {
            let mut ctx = $crate::error::ErrorContext::new($operation)
                .with_file(file!())
                .with_line(line!());
            $(
                ctx = ctx.with_info($key, $value);
            )+
            ctx
        }
    };
}

#[allow(dead_code)]
pub type Error = PyRunnerError;
