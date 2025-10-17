use std::fmt;
use thiserror::Error;

/// 项目统一错误类型
#[derive(Error, Debug)]
pub enum PyRunnerError {
    // === 任务执行相关错误 ===
    #[error("任务执行失败: {message}")]
    TaskExecutionFailed { message: String },

    #[error("任务超时: {task_id}")]
    TaskTimeout { task_id: String },

    #[error("任务被取消: {task_id}")]
    TaskCancelled { task_id: String },

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    // === Python相关错误 ===
    #[error("Python执行错误: {0}")]
    PythonError(String),

    #[error("Python变量未找到: {variable}")]
    PythonVariableNotFound { variable: String },

    #[error("Python模块导入失败: {module}")]
    PythonModuleImportFailed { module: String },

    // === JNI相关错误 ===
    #[error(transparent)]
    JniError(#[from] jni::errors::Error),

    #[error("JNI字符串转换失败")]
    JniStringConversionFailed,

    // === IO相关错误 ===
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("文件不存在: {path}")]
    FileNotFound { path: String },

    #[error("权限不足: {path}")]
    PermissionDenied { path: String },

    // === 序列化相关错误 ===
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    // === 进程相关错误 ===
    #[error("进程创建失败: {0}")]
    ProcessCreationFailed(String),

    #[error("进程执行失败: 退出码 {exit_code}")]
    ProcessExecutionFailed { exit_code: i32 },

    // === 系统相关错误 ===
    #[cfg(unix)]
    #[error(transparent)]
    NixError(#[from] nix::Error),

    #[error(transparent)]
    EnvVarError(#[from] std::env::VarError),

    // === 通信相关错误 ===
    #[error("消息发送失败: {0}")]
    MessageSendError(String),

    #[error("消息接收失败: {0}")]
    MessageReceiveError(String),

    #[error("通道已关闭")]
    ChannelClosed,

    // === 配置相关错误 ===
    #[error("配置错误: {message}")]
    ConfigError { message: String },

    #[error("参数无效: {parameter} = {value}")]
    InvalidParameter { parameter: String, value: String },

    // === 通用错误 ===
    #[error("内部错误: {message}")]
    InternalError { message: String },

    #[error("不支持的操作: {operation}")]
    UnsupportedOperation { operation: String },

    #[error("资源不足: {resource}")]
    ResourceExhausted { resource: String },

    #[error("超时: {operation}")]
    Timeout { operation: String },

    // === 透明错误包装 ===
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, PyRunnerError>;

impl PyRunnerError {
    /// 创建任务执行失败错误
    pub fn task_execution_failed<S: Into<String>>(message: S) -> Self {
        Self::TaskExecutionFailed {
            message: message.into(),
        }
    }

    /// 创建任务超时错误
    pub fn task_timeout<S: Into<String>>(task_id: S) -> Self {
        Self::TaskTimeout {
            task_id: task_id.into(),
        }
    }

    /// 创建Python错误
    pub fn python_error<S: Into<String>>(message: S) -> Self {
        Self::PythonError(message.into())
    }

    /// 创建Python变量未找到错误
    pub fn python_variable_not_found<S: Into<String>>(variable: S) -> Self {
        Self::PythonVariableNotFound {
            variable: variable.into(),
        }
    }

    /// 创建文件不存在错误
    pub fn file_not_found<S: Into<String>>(path: S) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// 创建权限不足错误
    pub fn permission_denied<S: Into<String>>(path: S) -> Self {
        Self::PermissionDenied { path: path.into() }
    }

    /// 创建配置错误
    pub fn config_error<S: Into<String>>(message: S) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// 创建内部错误
    pub fn internal_error<S: Into<String>>(message: S) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// 判断是否为可重试的错误
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

    /// 判断是否为致命错误
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::TaskCancelled { .. }
                | Self::PermissionDenied { .. }
                | Self::UnsupportedOperation { .. }
                | Self::ConfigError { .. }
        )
    }

    /// 获取错误代码（用于JNI返回）
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

/// 错误上下文信息
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

/// 带上下文的错误包装器
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

/// 扩展Result类型以支持上下文
pub trait ResultExt<T> {
    /// 为错误添加上下文信息
    fn with_context<F>(self, f: F) -> std::result::Result<T, ContextualError>
    where
        F: FnOnce() -> ErrorContext;

    /// 为错误添加操作上下文
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

/// 宏：快速创建带上下文的错误
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

/// 宏：快速创建错误上下文
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

// 为了向后兼容，保留Error类型别名
#[allow(dead_code)]
pub type Error = PyRunnerError;
