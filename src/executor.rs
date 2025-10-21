use crate::error::{PyRunnerError, Result};
use crate::listener::MessageListener;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info, instrument};

pub struct TaskExecutor {
    exec: String,
    argv: Vec<String>,
}

impl TaskExecutor {
    pub fn new(exec: String, argv: Vec<String>) -> Self {
        Self { exec, argv }
    }

    #[instrument(skip(self, listener))]
    pub async fn execute<L>(&self, mut listener: L) -> Result<()>
    where
        L: MessageListener,
    {
        info!("开始执行任务: exec: {}, argv: {:?}", self.exec, self.argv);

        let mut child = Command::new(&self.exec)
            .args(&self.argv)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        info!("子进程已创建: pid: {:?}", child.id());

        let mut stdout_lines =
            BufReader::new(child.stdout.take().ok_or_else(|| {
                PyRunnerError::ProcessCreationFailed("stdout is not piped".into())
            })?)
            .lines();
        let mut stderr_lines =
            BufReader::new(child.stderr.take().ok_or_else(|| {
                PyRunnerError::ProcessCreationFailed("stderr is not piped".into())
            })?)
            .lines();

        let mut stdout_done = false;
        let mut stderr_done = false;

        info!("开始读取子进程输出");
        while !(stdout_done && stderr_done) {
            tokio::select! {
                result = stdout_lines.next_line(), if !stdout_done => {
                    match result {
                        Ok(Some(line)) => listener.on_message(line),
                        Ok(None) => {
                            stdout_done = true;
                            info!("读取子进程stdout结束");
                        },
                        Err(e) => {
                            error!("读取子进程stdout失败: {e}");
                            return Err(PyRunnerError::IoError(e));
                        },
                    }
                }
                result = stderr_lines.next_line(), if !stderr_done => {
                    match result {
                        Ok(Some(line)) => listener.on_message(line),
                        Ok(None) => {
                            stderr_done = true;
                            info!("读取子进程stderr结束");
                        },
                        Err(e) => {
                            error!("读取子进程stderr失败: {e}");
                            return Err(PyRunnerError::IoError(e));
                        },
                    }
                }
            }
        }
        info!("读取子进程输出结束");

        info!("开始回收子进程");
        let status = child.wait().await?;
        if status.success() {
            info!("回收子进程成功: exit_status: {:?}", status);
        } else {
            error!("回收子进程失败: exit_status: {:?}", status);
            return Err(PyRunnerError::ProcessExecutionFailed(status));
        }

        Ok(())
    }
}
