use std::sync::Arc;
use std::time::Duration;

use spdlog::sink::{RotatingFileSink, RotationPolicy, StdStream, StdStreamSink};
use spdlog::{Level, LevelFilter, Logger, LoggerBuilder};

use crate::config::{Config, LogLevel};

impl From<LogLevel> for Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Critical => Level::Critical,
            LogLevel::Error => Level::Error,
            LogLevel::Warn => Level::Warn,
            LogLevel::Info => Level::Info,
            LogLevel::Debug => Level::Debug,
            LogLevel::Trace => Level::Trace,
        }
    }
}

fn add_console_sinks(builder: &mut LoggerBuilder) -> spdlog::Result<()> {
    let stdout = Arc::new(StdStreamSink::builder()
        .std_stream(StdStream::Stdout)
        .level_filter(LevelFilter::MoreVerbose(Level::Warn))
        .build()?);

    let stderr = Arc::new(StdStreamSink::builder()
        .std_stream(StdStream::Stderr)
        .level_filter(LevelFilter::MoreSevereEqual(Level::Warn))
        .build()?);

    builder.sink(stdout).sink(stderr);

    Ok(())
}

pub fn configure_logger(config: &Config) -> spdlog::Result<()> {
    if let Some(ref log) = config.log {
        let daily_sink = Arc::new(RotatingFileSink::builder()
            .base_path(log.location.as_ref().unwrap()) // required
            .rotation_policy(RotationPolicy::Daily { hour: 0, minute: 0 }) // required
            .max_files(60) // optional, defaults to `0` for no limit
            .rotate_on_open(false) // optional, defaults to `false`
            .build()?);

        let mut builder = Logger::builder();

        builder.sink(daily_sink);
        if log.log_to_console {
            add_console_sinks(&mut builder)?;
        }

        let daily_logger = Arc::new(builder.build()?);
        daily_logger.set_flush_level_filter(LevelFilter::MoreSevereEqual(Level::Info));
        daily_logger.set_flush_period(Some(Duration::from_secs(2)));
        daily_logger.set_level_filter(LevelFilter::MoreSevereEqual(log.level.into()));

        spdlog::set_default_logger(daily_logger);
    }

    Ok(())
}