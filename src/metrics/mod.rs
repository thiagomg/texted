use std::io;
use std::path::PathBuf;

use crate::metrics::access_metrics::AccessMetrics;
use crate::metrics::metric_publisher::MetricPublisher;

pub mod access_metrics;
pub mod metric_publisher;

mod naive_date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let s = date.format(FORMAT).to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

pub struct Metrics {
    access_metrics: AccessMetrics,
    metric_publisher: MetricPublisher,
}

impl Metrics {
    pub fn new(base_path: &PathBuf) -> spdlog::Result<Self> {
        let access_metrics = AccessMetrics::new();
        let metric_publisher = MetricPublisher::new(base_path)?;

        Ok(Self {
            access_metrics,
            metric_publisher,
        })
    }

    pub fn add(&mut self, post_name: &str, from: &str) -> io::Result<()> {
        self.access_metrics.add(post_name, from);
        if let Some(history) = self.access_metrics.remove_history() {
            self.metric_publisher.store_history(&history)?;
        }
        Ok(())
    }
}