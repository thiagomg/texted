use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::metrics::event_slot::EventSlot;
use spdlog::formatter::{pattern, PatternFormatter};
use spdlog::sink::{RotatingFileSink, RotationPolicy};
use spdlog::{info, Logger};

pub struct MetricPublisher {
    logger: Arc<Logger>,
}

impl MetricPublisher {
    pub fn new(base_path: &PathBuf) -> spdlog::Result<Self> {
        let pattern = pattern!("[SOR]{date} {time}.{millisecond}|{payload}[EOR]{eol}");
        let fmt = Box::new(PatternFormatter::new(pattern));

        let daily: Arc<RotatingFileSink> = Arc::new(
            RotatingFileSink::builder()
                .base_path(base_path)
                .rotation_policy(RotationPolicy::Daily { hour: 0, minute: 0 })
                .rotate_on_open(false)
                .formatter(fmt)
                .build()?,
        );

        let logger = Arc::new(Logger::builder().sink(daily).build()?);
        Ok(Self { logger })
    }

    pub fn store_events(&self, history: &Vec<EventSlot>) -> io::Result<()> {
        for event_slot in history {
            let json = serde_json::to_string(&event_slot)?;
            info!(logger: self.logger, "{}", &json);
            self.logger.flush();
        }

        Ok(())
    }
}
