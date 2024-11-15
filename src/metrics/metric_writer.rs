use crate::metrics::metric_aggregator::{Event, MetricAggregator};
use crate::metrics::metric_publisher::MetricPublisher;
use crate::metrics::metric_types::MetricEvent;
use chrono::{Duration, Utc};
use std::io;
use std::path::PathBuf;

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

    pub fn add_event(&mut self, metric_event: MetricEvent) -> io::Result<()> {
        let event = Event {
            metric_event,
            date_time: Utc::now(),
            total: 1,
        };
        self.metric_aggregator.add_event(event);
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
