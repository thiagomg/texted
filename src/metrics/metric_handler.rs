use crate::metrics::metric_sender::MetricSender;
use crate::metrics::metric_types::MetricEvent;
use crate::metrics::metric_writer::MetricWriter;
use spdlog::{error, info, trace};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

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
                        if let Err(e) = metrics.add_event(event) {
                            error!("Error writing access metric: {}", e);
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

    pub fn new_sender(&self) -> MetricSender {
        MetricSender::new(self.sender.clone())
    }

    pub fn no_op() -> MetricSender {
        MetricSender::no_op()
    }
}
