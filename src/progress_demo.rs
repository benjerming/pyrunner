use crate::error::Result;
use crate::progress_monitor::{
    ConsoleProgressListener, ProcessTaskExecutor, ProgressListener, ThreadTaskExecutor,
    run_task_with_monitoring,
};
use log::error;

pub fn demo_thread_task() {
    println!("🧵 开始演示子线程任务执行\n");

    let task_fn = || -> Result<()> {
        use std::thread;
        use std::time::Duration;

        for i in 1..=5 {
            thread::sleep(Duration::from_millis(200));
            println!("执行步骤 {}/5", i);
        }

        println!("线程任务执行成功");
        Ok(())
    };

    let executor = ThreadTaskExecutor::new(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>];

    match run_task_with_monitoring("thread_task".to_string(), executor, listeners) {
        Ok(_) => println!("\n✅ 子线程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

pub fn demo_process_task() {
    println!("🐍 开始演示子进程任务执行\n");

    let task_fn = || -> Result<()> {
        use std::thread;
        use std::time::Duration;

        for i in 1..=5 {
            thread::sleep(Duration::from_millis(200));
            println!("执行步骤 {}/5", i);
        }

        println!("任务执行成功");
        Ok(())
    };

    let executor = ProcessTaskExecutor::new(task_fn);
    let listeners = vec![Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>];

    match run_task_with_monitoring("process_task".to_string(), executor, listeners) {
        Ok(_) => println!("\n✅ 子进程任务执行演示完成"),
        Err(e) => error!("任务执行失败: {}", e),
    }
}

pub fn run_all_demos() {
    println!("🎯 开始运行进度监控演示\n");
    println!("{}", "=".repeat(60));

    demo_thread_task();

    println!("\n{}", "=".repeat(60));

    demo_process_task();

    println!("\n{}", "=".repeat(60));

    println!("\n🎊 所有演示完成！");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_thread_task() {
        demo_thread_task();
    }

    #[test]
    fn test_demo_process_task() {
        demo_process_task();
    }
}
