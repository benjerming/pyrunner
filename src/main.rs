use env_logger;
use log::{LevelFilter, error, info};
use std::env;

// 引入错误处理模块用于演示

mod error;
// mod demo;
// mod executor;
mod jni;
mod message_receiver;
mod message_sender;
mod progress_demo;
mod progress_monitor;
// mod statements;
mod task_executor;

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
                progress_demo::demo_thread_task();
            }
            "process" | "p" => {
                info!("运行子进程任务演示");
                progress_demo::demo_process_task();
            }
            "all" | "a" => {
                info!("运行所有演示");
                progress_demo::run_all_demos();
            }
            _ => {
                error!("无效的任务类型: {task}");
                print_usage();
            }
        },
        None => {
            println!("运行所有演示");
            progress_demo::run_all_demos();
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
