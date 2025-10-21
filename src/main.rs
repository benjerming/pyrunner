use std::env;
use tracing::{Span, error, info, instrument};

mod error;
mod ipc;
mod jni;
mod executor;

use ipc::MessageSender;
use ipc::ConsoleProgressListener;
use executor::TaskExecutor;

use std::sync::{Arc, Mutex};


fn init_logger() {
    use tracing_indicatif::filter::IndicatifFilter;
    use tracing_indicatif::style::ProgressStyle;
    use tracing_subscriber::{
        EnvFilter, layer::SubscriberExt, prelude::*, util::SubscriberInitExt,
    };

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let indicatif_layer = tracing_indicatif::IndicatifLayer::new().with_progress_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(indicatif_layer.get_stdout_writer()))
        .with(indicatif_layer.with_filter(IndicatifFilter::new(false)))
        .init();
}

fn main() {
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

    for i in 1..=40 {
        thread::sleep(Duration::from_millis(40));
        sender.send_task_progress(task_id, i, 40);
    }

    sender.send_task_completed(task_id);
    Ok(())
}

#[instrument(fields(indicatif.pb_show = tracing::field::Empty))]
fn demo_thread_task() {
    info!("🧵 开始演示子线程任务执行");

    let task_id = 1;
    let executor = TaskExecutor::new_thread(task_fn);
    let listener = Arc::new(Mutex::new(ConsoleProgressListener::new(task_id, Span::current())));

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(executor.run_with_monitoring(task_id, listener)) {
        Ok(_) => info!("✅ 子线程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

#[instrument(fields(indicatif.pb_show = tracing::field::Empty))]
fn demo_process_task() {
    info!("🐍 开始演示子进程任务执行");

    let task_id = 2;
    let executor = TaskExecutor::new_process(task_fn);
    let listener = Arc::new(Mutex::new(ConsoleProgressListener::new(task_id, Span::current())));

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(executor.run_with_monitoring(task_id, listener)) {
        Ok(_) => info!("✅ 子进程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

fn demo_all_tasks() {
    info!("🎯 开始运行进度监控演示");
    info!("{}", "=".repeat(60));

    demo_thread_task();

    info!("{}", "=".repeat(60));

    demo_process_task();

    info!("{}", "=".repeat(60));

    info!("🎊 所有演示完成！");
}
