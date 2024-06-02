use std::io;
use std::path::PathBuf;

use spdlog::{debug, error};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

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

pub struct MetricWriter {
    access_metrics: AccessMetrics,
    metric_publisher: MetricPublisher,
}

impl MetricWriter {
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

// -----------

pub struct MetricEvent {
    pub post_name: String,
    pub origin: String,
}

pub struct MetricHandler {
    _receiver_task: JoinHandle<()>,
    sender: Sender<MetricEvent>,
}

impl MetricHandler {
    pub fn new(mut metrics: MetricWriter) -> Self {
        let (tx, mut rx) = mpsc::channel::<MetricEvent>(64);

        let receiver_task = tokio::spawn(async move {
            println!("Starting metrics receiver");
            while let Some(event) = rx.recv().await {
                if let Err(e) = metrics.add(&event.post_name, &event.origin) {
                    error!("Error writing access metric for {}: {}", &event.post_name, e);
                } else {
                    debug!("Metric event written for {}", &event.post_name);
                }
            }
        });

        Self {
            _receiver_task: receiver_task,
            sender: tx,
        }
    }

    pub fn new_sender(&self) -> Sender<MetricEvent> {
        self.sender.clone()
    }
}