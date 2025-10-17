# PyRunner - 进度监控系统

一个用Rust实现的简洁进度监控系统，支持子线程和子进程两种方式执行耗时任务，并实时监控任务进度。

## 功能特性

- 🧵 **子线程任务执行**: 在子线程中执行耗时任务，主线程监控进度
- 🐍 **子进程任务执行**: 在子进程中执行Python脚本，实时捕获输出
- 📊 **可视化进度条**: 美观的控制台进度条显示
- 🎯 **简洁的监听器系统**: 支持自定义进度监听器
- ⚡ **异步支持**: 基于Tokio的异步执行框架
- 🛡️ **类型安全**: 利用Rust的类型系统确保内存安全

## 项目结构

```
src/
├── lib.rs                  # 库入口
├── main.rs                 # 主程序入口
├── message_sender.rs       # 消息发送器模块
├── message_receiver.rs     # 消息接收器模块  
├── task_executor.rs        # 任务执行器模块
├── progress_monitor.rs     # 进度监控高级封装模块
├── progress_demo.rs        # 演示程序
├── demo_progress.py       # Python演示脚本
└── ...                    # 其他模块
```

## 核心组件

### 1. MessageSender - 消息发送器

负责发送进度信息：

```rust
pub struct MessageSender {
    // 内部实现...
}

impl MessageSender {
    pub fn send_task_started(&self, task_id: String);
    pub fn send_task_progress(&self, task_id: String, percentage: f64, message: String);
    pub fn send_task_completed(&self, task_id: String);
    pub fn send_task_error(&self, task_id: String, error_msg: String);
}
```

### 2. MessageReceiver - 消息接收器

负责接收和处理进度消息：

```rust
pub struct MessageReceiver {
    // 内部实现...
}

pub trait ProgressListener: Send + Sync {
    fn on_progress_update(&self, progress: &ProgressInfo);
    fn on_task_completed(&self, progress: &ProgressInfo);
    fn on_task_error(&self, progress: &ProgressInfo);
}

// 内置监听器
pub struct ConsoleProgressListener;  // 控制台输出
```

### 3. TaskExecutor - 任务执行器

定义任务执行接口和实现：

```rust
pub trait TaskExecutor: Send + Sync {
    fn execute(&self, task_id: String, sender: &MessageSender) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

// 内置执行器
pub struct ThreadTaskExecutor;       // 子线程任务执行器
pub struct ProcessTaskExecutor;      // 子进程任务执行器（Python脚本）
```

### 4. ProgressInfo - 进度信息结构体

```rust
pub struct ProgressInfo {
    pub task_id: String,        // 任务ID
    pub percentage: f64,        // 进度百分比 (0-100)
    pub message: String,        // 进度描述信息
    pub current_step: u32,      // 当前步骤
    pub total_steps: u32,       // 总步骤数
    pub is_completed: bool,     // 是否完成
    pub has_error: bool,        // 是否出错
    pub error_message: Option<String>, // 错误信息
}
```

## 使用方法

### 编译项目

```bash
cargo build
```

### 运行演示

```bash
# 运行所有演示
cargo run --bin pyrunner_demo

# 运行子线程任务演示
cargo run --bin pyrunner_demo thread

# 运行子进程任务演示
cargo run --bin pyrunner_demo process
```

### 基本使用示例

#### 子线程任务执行

```rust
use pyrunner::progress_monitor::{
    run_task_with_monitoring, ConsoleProgressListener, 
    ThreadTaskExecutor, ProgressListener
};

// 创建任务执行器
let executor = ThreadTaskExecutor::new(10, 20); // 10秒，20步

// 创建监听器
let listeners = vec![
    Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>
];

// 运行任务并监控进度
match run_task_with_monitoring("my_task".to_string(), executor, listeners) {
    Ok(_) => println!("任务完成"),
    Err(e) => println!("任务失败: {}", e),
}
```

#### 子进程任务执行

```rust
use pyrunner::progress_monitor::{
    run_task_with_monitoring, ConsoleProgressListener, 
    ProcessTaskExecutor, ProgressListener
};

// 创建任务执行器（不再启动子进程）
let task_fn = || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::thread;
    use std::time::Duration;
    
    // 模拟一些计算工作
    for i in 1..=10 {
        thread::sleep(Duration::from_millis(100));
        println!("处理步骤 {}/10", i);
    }
    
    println!("任务完成");
    Ok(())
};

let executor = ProcessTaskExecutor::new(task_fn);

// 创建监听器
let listeners = vec![
    Box::new(ConsoleProgressListener) as Box<dyn ProgressListener>
];

// 运行任务并监控进度
match run_task_with_monitoring("python_task".to_string(), executor, listeners) {
    Ok(_) => println!("Python脚本执行完成"),
    Err(e) => println!("Python脚本执行失败: {}", e),
}
```

#### 手动组装组件

```rust
use pyrunner::message_sender::create_message_channel;
use pyrunner::message_receiver::{MessageReceiver, ConsoleProgressListener};
use pyrunner::task_executor::{ThreadTaskExecutor, TaskExecutor};
use std::thread;

// 创建消息通道
let (sender, receiver) = create_message_channel();

// 创建消息接收器并添加监听器
let mut message_receiver = MessageReceiver::new(receiver);
message_receiver.add_listener(Box::new(ConsoleProgressListener));

// 在子线程中启动监听
let monitor_handle = thread::spawn(move || {
    message_receiver.start_listening();
});

// 创建并执行任务
let executor = ThreadTaskExecutor::new(5, 10);
let result = executor.execute("my_task".to_string(), &sender);

// 等待完成
monitor_handle.join().unwrap();
```

## 演示效果

### 子线程任务执行
```
🧵 开始演示子线程任务执行

[█████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 10.0% - 正在执行步骤 1/10 (10/100)
[██████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 20.0% - 正在执行步骤 2/10 (20/100)
...
[██████████████████████████████████████████████████] 100.0% - 正在执行步骤 10/10 (100/100)

✅ 任务完成: thread_task - 任务完成
```

### 子进程任务执行
```
🐍 开始演示子进程任务执行

[█████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 10.0% - 输出: 开始执行耗时任务...
[██████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 20.0% - 输出: 进度: 5.0% - 正在处理步骤 1/20
...
✅ 任务完成: process_task - 任务完成
```

## 技术特点

- **🏗️ 模块化设计**: 消息发送器、接收器、任务执行器独立模块，职责清晰
- **📡 进程间通信**: 使用Rust的`mpsc`通道进行线程间通信
- **⚡ 异步支持**: 基于Tokio异步运行时，支持同步和异步任务
- **🛡️ 类型安全**: 利用Rust的类型系统确保内存安全和线程安全
- **🔧 错误处理**: 使用`Result`类型进行错误传播
- **🔌 可扩展性**: 通过trait系统支持自定义监听器和任务执行器

## 依赖项

- `tokio`: 异步运行时
- `serde`: 序列化支持
- `log`: 日志记录
- `env_logger`: 环境日志
- `thiserror`: 错误处理

## 许可证

MIT License