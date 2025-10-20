use env_logger;
use log::{LevelFilter, error, info};
use std::env;

// 引入错误处理模块用于演示

mod error;
mod jni;
mod message_receiver;
mod message_sender;
mod progress_monitor;
mod task_executor;

use crate::error::Result;
use crate::progress_monitor::{
    ConsoleProgressListener, MessageListener, ProcessTaskExecutor, ThreadTaskExecutor,
    run_task_with_monitoring,
};

fn init_logger() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
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

fn demo_thread_task() {
    println!("🧵 开始演示子线程任务执行\n");

    let task_fn = |sender: &crate::message_sender::MessageSender| -> Result<()> {
        use std::thread;
        use std::time::Duration;

        for i in 1..=5 {
            thread::sleep(Duration::from_millis(200));
            let percentage = (i as f64 / 5.0) * 100.0;
            sender.send_task_progress(
                "thread_task".to_string(),
                percentage,
                format!("执行步骤 {}/5", i),
            );
            println!("执行步骤 {}/5", i);
        }

        println!("线程任务执行成功");
        Ok(())
    };

    let executor = ThreadTaskExecutor::new(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn MessageListener>];

    match run_task_with_monitoring("thread_task".to_string(), executor, listeners) {
        Ok(_) => println!("\n✅ 子线程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

fn demo_process_task() {
    println!("🐍 开始演示子进程任务执行\n");

    let task_fn = |sender: &crate::message_sender::MessageSender| -> Result<()> {
        use std::thread;
        use std::time::Duration;

        for i in 1..=5 {
            thread::sleep(Duration::from_millis(200));
            let percentage = (i as f64 / 5.0) * 100.0;
            sender.send_task_progress(
                "process_task".to_string(),
                percentage,
                format!("执行步骤 {}/5", i),
            );
            println!("执行步骤 {}/5", i);
        }

        println!("任务执行成功");
        Ok(())
    };

    let executor = ProcessTaskExecutor::new(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn MessageListener>];

    match run_task_with_monitoring("process_task".to_string(), executor, listeners) {
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
