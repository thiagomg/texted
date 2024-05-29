use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use spdlog::{info, Logger};
use spdlog::sink::{RotatingFileSink, RotationPolicy};

use crate::metrics::access_metrics::PostCounter;

pub struct MetricPublisher {
    logger: Arc<Logger>,
}

impl MetricPublisher {
    pub fn new(base_path: &PathBuf) -> spdlog::Result<Self> {
        let daily: Arc<RotatingFileSink> = Arc::new(
            RotatingFileSink::builder()
                .base_path(base_path)
                .rotation_policy(RotationPolicy::Daily { hour: 0, minute: 0 })
                .rotate_on_open(false)
                .build()?
        );

        let logger = Arc::new(Logger::builder().sink(daily).build()?);
        Ok(Self {
            logger,
        })
    }

    pub fn store_history(&self, history: &Vec<PostCounter>) -> io::Result<()> {
        for post_counter in history {
            let json = serde_json::to_string(&post_counter)?;
            info!(logger: self.logger, "{}", &json);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::metrics::access_metrics::AccessMetrics;

    use super::*;

    #[test]
    fn write_to_file() {
        let d = || -> NaiveDate {
            static mut COUNTER: u32 = 0;
            let day = unsafe {
                COUNTER += 1;
                match COUNTER {
                    0 | 1 => 21,
                    2 | 3 => 22,
                    _ => 23,
                }
            };

            NaiveDate::from_ymd_opt(2024, 05, day).unwrap()
        };

        let mut metrics = AccessMetrics::new_for_test(Box::new(d));

        let publisher = MetricPublisher {
            logger: spdlog::default_logger(),
        };

        metrics.add("post-1", "10.0.0.1");
        metrics.add("post-1", "10.0.0.2");
        metrics.add("post-1", "10.0.0.1");
        metrics.add("post-1", "10.0.0.2");
        metrics.add("post-1", "10.0.0.1");
        metrics.add("post-1", "10.0.0.2");
        metrics.add("post-2", "10.0.0.1");
        metrics.add("post-2", "10.0.0.2");

        publisher.store_history(&metrics.remove_history().unwrap()).unwrap();
    }
}