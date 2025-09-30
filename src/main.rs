use env_logger;
use log::{LevelFilter, info};
use std::env;

mod async_executor;
mod demo;
mod executor;
mod jni;
mod message_receiver;
mod message_sender;
mod progress_demo;
mod progress_monitor;
mod statements;
mod sync_executor;
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

    // 解析命令行参数
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "thread" => {
                println!("运行子线程任务演示");
                progress_demo::demo_thread_task();
            }
            "process" => {
                println!("运行子进程任务演示");
                progress_demo::demo_process_task();
            }
            _ => {
                println!("运行所有演示");
                progress_demo::run_all_demos();
            }
        }
    } else {
        println!("运行默认演示（所有演示）");
        progress_demo::run_all_demos();
    }

    info!("程序执行完成");
}

fn print_usage() {
    println!("用法: cargo run [选项]");
    println!("选项:");
    println!("  thread     - 运行子线程任务演示");
    println!("  process    - 运行子进程任务演示");
    println!("             - 运行所有演示（默认）");
}
