use crate::message_receiver::MessageReceiver;
use crate::message_sender::create_message_channel;
use crate::progress_monitor::{
    ConsoleProgressListener, ProgressInfo, ProgressListener,
    run_task_with_monitoring, ThreadTaskExecutor, ProcessTaskExecutor,
};
use log::error;
use std::thread;
use std::time::Duration;

/// 演示子线程任务执行
pub fn demo_thread_task() {
    println!("🧵 开始演示子线程任务执行\n");

    let executor = ThreadTaskExecutor::new(1, 10); // 1秒，10步
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>];

    match run_task_with_monitoring("thread_task".to_string(), executor, listeners) {
        Ok(_) => println!("\n✅ 子线程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

/// 演示子进程任务执行
pub fn demo_process_task() {
    println!("🐍 开始演示子进程任务执行\n");

    let executor = ProcessTaskExecutor::new("src/demo_progress.py".to_string());
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>];

    match run_task_with_monitoring("process_task".to_string(), executor, listeners) {
        Ok(_) => println!("\n✅ 子进程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}


/// 运行所有演示
pub fn run_all_demos() {
    println!("🎯 开始运行进度监控演示\n");
    println!("{}", "=".repeat(60));

    // 子线程任务演示
    demo_thread_task();

    println!("\n{}", "=".repeat(60));

    // 子进程任务演示
    demo_process_task();

    println!("\n{}", "=".repeat(60));

    println!("\n🎊 所有演示完成！");
}

