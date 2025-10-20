use std::env;
use tracing::{error, info, instrument};

// 引入错误处理模块用于演示

mod error;
mod ipc;
mod jni;
mod progress_monitor;
mod task_executor;

use ipc::MessageSender;
use progress_monitor::{
    ConsoleProgressListener, MessageListener, TaskExecutor, run_task_with_monitoring,
};

fn init_logger() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
        .init();
}

fn main() {
    // 初始化日志
    init_logger();

    info!("启动 PyRunner 进度监控演示程序");

    match env::args().nth(1) {
        Some(task) => match task.to_ascii_lowercase().as_str() {
            "thread" | "t" => {
                info!("运行子线程任务演示");
                demo_thread_task();
            }
            "process" | "p" => {
                info!("运行子进程任务演示");
                demo_process_task();
            }
            "all" | "a" => {
                info!("运行所有演示");
                demo_all_tasks();
            }
            _ => {
                error!("无效的任务类型: {task}");
                print_usage();
            }
        },
        None => {
            info!("运行所有演示");
            demo_all_tasks();
        }
    }

    info!("程序执行完成");
}

fn print_usage() {
    info!("用法: cargo run [选项]");
    info!("选项:");
    info!("  thread     - 运行子线程任务演示");
    info!("  process    - 运行子进程任务演示");
    info!("  all        - 运行所有演示（默认）");
}

fn task_fn(sender: &MessageSender, task_id: u64) -> std::result::Result<(), error::PyRunnerError> {
    use std::thread;
    use std::time::Duration;

    sender.send_task_started(task_id);

    for i in 1..=100 {
        thread::sleep(Duration::from_millis(100));
        sender.send_task_progress(task_id, i, 100);
    }

    // info!("任务执行成功");
    sender.send_task_completed(task_id);
    Ok(())
}

#[instrument]
fn demo_thread_task() {
    info!("🧵 开始演示子线程任务执行\n");

    let executor = TaskExecutor::new_thread(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener::new()) as Box<dyn MessageListener>];

    match run_task_with_monitoring(1, executor, listeners) {
        Ok(_) => info!("\n✅ 子线程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

fn demo_process_task() {
    info!("🐍 开始演示子进程任务执行\n");

    let executor = TaskExecutor::new_process(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener::new()) as Box<dyn MessageListener>];

    match run_task_with_monitoring(2, executor, listeners) {
        Ok(_) => info!("\n✅ 子进程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

fn demo_all_tasks() {
    info!("🎯 开始运行进度监控演示\n");
    info!("{}", "=".repeat(60));

    demo_thread_task();

    info!("\n{}", "=".repeat(60));

    demo_process_task();

    info!("\n{}", "=".repeat(60));

    info!("\n🎊 所有演示完成！");
}
