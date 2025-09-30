use crate::message_sender::{MessageSender, ProgressInfo};
use log::{debug, error, info};
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// 任务执行器trait
pub trait TaskExecutor: Send + Sync {
    /// 执行任务
    fn execute(
        &self,
        task_id: String,
        sender: &MessageSender,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// 子线程任务执行器 - 在子线程中执行耗时任务
pub struct ThreadTaskExecutor {
    duration_seconds: u64,
    steps: u32,
}

impl ThreadTaskExecutor {
    /// 创建新的子线程任务执行器
    pub fn new(duration_seconds: u64, steps: u32) -> Self {
        Self {
            duration_seconds,
            steps: steps.max(1), // 确保至少有1步
        }
    }
}

impl TaskExecutor for ThreadTaskExecutor {
    fn execute(
        &self,
        task_id: String,
        sender: &MessageSender,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("开始执行子线程任务: {}", task_id);

        // 发送任务开始消息
        sender.send_task_started(task_id.clone());

        let step_duration = Duration::from_millis(self.duration_seconds * 1000 / self.steps as u64);

        for i in 1..=self.steps {
            thread::sleep(step_duration);

            let percentage = (i as f64 / self.steps as f64) * 100.0;
            let message = format!("正在执行步骤 {}/{}", i, self.steps);

            sender.send_task_progress(task_id.clone(), percentage, message);
        }

        // 发送任务完成消息
        sender.send_task_completed(task_id.clone());
        info!("子线程任务执行完成: {}", task_id);

        Ok(())
    }
}

/// 子进程任务执行器 - 在子进程中执行Python脚本
pub struct ProcessTaskExecutor {
    script_path: String,
}

impl ProcessTaskExecutor {
    /// 创建新的子进程任务执行器
    pub fn new(script_path: String) -> Self {
        Self { script_path }
    }

    /// 异步执行Python脚本
    pub async fn execute_async(
        &self,
        task_id: String,
        sender: &MessageSender,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "开始执行Python脚本: {} (任务ID: {})",
            self.script_path, task_id
        );

        sender.send_task_progress(task_id.clone(), 0.0, "启动Python脚本".to_string());

        let mut child = Command::new("python")
            .arg(&self.script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);

            let sender_clone = sender.clone();
            let task_id_clone = task_id.clone();

            tokio::spawn(async move {
                let mut lines = reader.lines();
                let mut step = 0;

                while let Ok(Some(line)) = lines.next_line().await {
                    step += 1;
                    let percentage = (step as f64 * 10.0).min(90.0);

                    sender_clone.send_task_progress(
                        task_id_clone.clone(),
                        percentage,
                        format!("输出: {}", line),
                    );
                }
            });
        }

        let status = child.wait().await?;

        if status.success() {
            sender.send_task_completed(task_id.clone());
            info!("Python脚本执行成功: {}", task_id);
        } else {
            let error_msg = format!("Python脚本执行失败，退出码: {:?}", status.code());
            sender.send_task_error(task_id.clone(), error_msg.clone());
            error!("Python脚本执行失败: {} - {}", task_id, error_msg);
            return Err(error_msg.into());
        }

        Ok(())
    }
}

impl TaskExecutor for ProcessTaskExecutor {
    fn execute(
        &self,
        task_id: String,
        sender: &MessageSender,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.execute_async(task_id, sender))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_sender::create_message_channel;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_thread_task_executor() {
        let (sender, receiver) = create_message_channel();
        let executor = ThreadTaskExecutor::new(1, 5); // 1秒，5步

        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        // 在另一个线程中收集消息
        thread::spawn(move || {
            while let Ok(msg) = receiver.recv() {
                let mut msgs = messages_clone.lock().unwrap();
                msgs.push(msg);
                if msgs.last().unwrap().is_completed || msgs.last().unwrap().has_error {
                    break;
                }
            }
        });

        // 执行任务
        let result = executor.execute("test_thread".to_string(), &sender);
        assert!(result.is_ok());

        // 等待消息收集完成
        thread::sleep(Duration::from_millis(100));

        let messages = messages.lock().unwrap();
        assert!(messages.len() >= 6); // 开始 + 5步 + 完成
        assert!(messages.last().unwrap().is_completed);
    }
}
