use tracing::{Span, error, info, instrument};

mod error;
mod executor;
mod ipc;
mod jni;
mod listener;

use executor::TaskExecutor;

use crate::listener::ConsoleProgressListener;

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

#[tokio::main]
async fn main() {
    init_logger();
    demo_process_task().await;
}

#[instrument(fields(indicatif.pb_show = tracing::field::Empty))]
async fn demo_process_task() {
    info!("开始执行任务");

    let task_id = 2;
    let executor = TaskExecutor::new("python".into(), vec!["src/demo_progress.py".into()]);
    let mut listener = ConsoleProgressListener::new(task_id, Span::current());

    match executor.execute(&mut listener).await {
        Ok(_) => info!("✅ 任务执行成功"),
        Err(e) => error!("❌ 任务执行失败: {}", e),
    }
}
