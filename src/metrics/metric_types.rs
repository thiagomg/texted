pub struct PostDetail {
    pub post_name: String,
}

pub struct PageDetail {
    pub page_name: String,
}

pub struct ListDetail {
    pub tag: Option<String>,
}
pub enum EventApi {
    View(PostDetail),
    Page(PageDetail),
    List(ListDetail),
    Index,
    Rss,
}

pub struct MetricEvent {
    pub api: EventApi,
    pub origin: String,
}
