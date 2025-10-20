use tracing::{error, info};
use std::env;

// 引入错误处理模块用于演示

mod error;
mod jni;
mod message_receiver;
mod message_sender;
mod progress_monitor;
mod task_executor;

use message_sender::MessageSender;
use progress_monitor::{
    ConsoleProgressListener, MessageListener, TaskExecutor, run_task_with_monitoring,
};

fn init_logger() {
    use tracing_subscriber::EnvFilter;
    
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
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
            println!("运行所有演示");
            demo_all_tasks();
        }
    }

    info!("程序执行完成");
}

fn print_usage() {
    println!("用法: cargo run [选项]");
    println!("选项:");
    println!("  thread     - 运行子线程任务演示");
    println!("  process    - 运行子进程任务演示");
    println!("  error      - 运行错误处理演示");
    println!("  all        - 运行所有演示（默认）");
}

fn task_fn(sender: &MessageSender, task_id: u64) -> std::result::Result<(), error::PyRunnerError> {
    use std::thread;
    use std::time::Duration;

    sender.send_task_started(task_id);

    for i in 1..=100 {
        thread::sleep(Duration::from_millis(100));
        let percentage = (i as f64 / 100.0) * 100.0;
        sender.send_task_progress(task_id, percentage, format!("执行步骤 {}/5", i));
        // println!("执行步骤 {}/5", i);
    }

    // println!("任务执行成功");
    sender.send_task_completed(task_id);
    Ok(())
}

fn demo_thread_task() {
    println!("🧵 开始演示子线程任务执行\n");

    let executor = TaskExecutor::new_thread(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn MessageListener>];

    match run_task_with_monitoring(1, executor, listeners) {
        Ok(_) => println!("\n✅ 子线程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

fn demo_process_task() {
    println!("🐍 开始演示子进程任务执行\n");

    let executor = TaskExecutor::new_process(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn MessageListener>];

    match run_task_with_monitoring(2, executor, listeners) {
        Ok(_) => println!("\n✅ 子进程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

fn demo_all_tasks() {
    println!("🎯 开始运行进度监控演示\n");
    println!("{}", "=".repeat(60));

    demo_thread_task();

    println!("\n{}", "=".repeat(60));

    demo_process_task();

    println!("\n{}", "=".repeat(60));

    println!("\n🎊 所有演示完成！");
}
