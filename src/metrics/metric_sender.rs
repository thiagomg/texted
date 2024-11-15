use crate::metrics::metric_types::EventApi::{Index, List, Page, Rss, View};
use crate::metrics::metric_types::{ListDetail, MetricEvent, PageDetail, PostDetail};
use spdlog::error;
use tokio::sync::mpsc::Sender;

pub struct MetricSender {
    sender_ch: Option<Sender<MetricEvent>>,
}

impl MetricSender {
    pub fn new(sender_ch: Sender<MetricEvent>) -> Self {
        Self {
            sender_ch: Some(sender_ch),
        }
    }

    pub fn no_op() -> Self {
        Self { sender_ch: None }
    }

    pub async fn view(&self, post_name: String, origin: String) {
        if let Some(ref sender) = self.sender_ch {
            if let Err(e) = sender
                .send(MetricEvent {
                    api: View(PostDetail { post_name }),
                    origin,
                })
                .await
            {
                error!("Error writing view metrics: {}", e);
            }
        }
    }

    pub async fn page(&self, page_name: String, origin: String) {
        if let Some(ref sender) = self.sender_ch {
            if let Err(e) = sender
                .send(MetricEvent {
                    api: Page(PageDetail { page_name }),
                    origin,
                })
                .await
            {
                error!("Error writing page metrics: {}", e);
            }
        }
    }

    pub async fn list(&self, tag: Option<String>, origin: String) {
        if let Some(ref sender) = self.sender_ch {
            if let Err(e) = sender
                .send(MetricEvent {
                    api: List(ListDetail { tag }),
                    origin,
                })
                .await
            {
                error!("Error writing list metrics: {}", e);
            }
        }
    }

    pub async fn rss(&self, origin: String) {
        if let Some(ref sender) = self.sender_ch {
            if let Err(e) = sender.send(MetricEvent { api: Rss, origin }).await {
                error!("Error writing rss metrics: {}", e);
            }
        }
    }

    pub async fn index(&self, origin: String) {
        if let Some(ref sender) = self.sender_ch {
            if let Err(e) = sender.send(MetricEvent { api: Index, origin }).await {
                error!("Error writing index metrics: {}", e);
            }
        }
    }
}
