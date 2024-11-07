use crate::metrics::metric_aggregator::MetricAggregator;
use crate::metrics::metric_publisher::MetricPublisher;
use chrono::Duration;
use spdlog::{debug, error, info, trace};
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

pub mod metric_publisher;
mod metric_aggregator;
mod event_slot;

pub struct MetricWriter {
    metric_aggregator: MetricAggregator,
    metric_publisher: MetricPublisher,
}

impl MetricWriter {
    pub fn new(base_path: &PathBuf, time_slot: Duration) -> spdlog::Result<Self> {
        let metric_aggregator = MetricAggregator::new(time_slot);
        let metric_publisher = MetricPublisher::new(base_path)?;

        Ok(Self {
            metric_aggregator,
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

    pub fn flush(&mut self) -> io::Result<()> {
        self.metric_aggregator.flush();
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
            info!("Starting metrics receiver");
            loop {
                match tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv()).await {
                    Ok(Some(event)) => {
                        if let Err(e) = metrics.add(&event.post_name, &event.origin) {
                            error!("Error writing access metric for {}: {}", &event.post_name, e);
                        } else {
                            debug!("Metric event written for {}", &event.post_name);
                        }
                    }
                    Ok(None) => break,
                    Err(_timeout) => {
                        if let Err(e) = metrics.flush() {
                            error!("Error flushing access metric: {}", e);
                        }
                        trace!("Timeout - flushing metrics");
                    }
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