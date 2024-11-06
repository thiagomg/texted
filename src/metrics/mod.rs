use chrono::Duration;
use spdlog::{debug, error};
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use crate::metrics::metric_aggregator::MetricAggregator;
use crate::metrics::metric_publisher::MetricPublisher;

pub mod metric_publisher;
mod metric_aggregator;
mod event_slot;

pub struct MetricWriter {
    metric_aggregator: MetricAggregator,
    metric_publisher: MetricPublisher,
}

impl MetricWriter {
    pub fn new(base_path: &PathBuf) -> spdlog::Result<Self> {
        let access_metrics = MetricAggregator::new(Duration::minutes(1));
        let metric_publisher = MetricPublisher::new(base_path)?;

        Ok(Self {
            metric_aggregator: access_metrics,
            metric_publisher,
        })
    }

    pub fn add(&mut self, post_name: &str, from: &str) -> io::Result<()> {
        self.metric_aggregator.add(post_name, from);
        if let Some(history) = self.metric_aggregator.take_events() {
            self.metric_publisher.store_events(&history)?;
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