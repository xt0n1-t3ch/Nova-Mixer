use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init() -> Option<WorkerGuard> {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| "novamixer=info".into());
    let console = fmt::layer().compact();
    let logs = crate::paths::default_data_dir()?.join("logs");
    std::fs::create_dir_all(&logs).ok()?;
    let appender = tracing_appender::rolling::daily(logs, "novamixer.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let file = fmt::layer().with_ansi(false).with_writer(writer);
    tracing_subscriber::registry()
        .with(filter())
        .with(console)
        .with(file)
        .try_init()
        .ok();
    Some(guard)
}
