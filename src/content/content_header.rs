use std::path::PathBuf;

use chrono::NaiveDateTime;

use crate::post::PostId;

#[derive(Debug, Clone)]
pub struct ContentHeader {
    pub file_name: PathBuf,
    pub id: PostId,
    pub date: NaiveDateTime,
    pub author: String,
    pub tags: Vec<String>,
}