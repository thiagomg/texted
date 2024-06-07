use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::metrics::naive_date_format;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostCounter {
    pub post_id: String,
    pub total: u64,
    pub origins: HashSet<String>,
    #[serde(with = "naive_date_format")]
    pub stats_date: NaiveDate,
}

pub trait DateProvider {
    fn today(&self) -> NaiveDate;
}

/// We have unique visits per day and history of past days, all separated per link
pub struct AccessMetrics {
    /// Post name -> Post Counter
    post_counter: HashMap<String, PostCounter>,
    history: Vec<PostCounter>,
    date_provider: Box<dyn Fn() -> NaiveDate + Send>,
}

struct TodayProvider {}

impl DateProvider for TodayProvider {
    fn today(&self) -> NaiveDate {
        Utc::now().date_naive()
    }
}

impl AccessMetrics {
    pub fn new() -> Self {
        Self {
            post_counter: Default::default(),
            history: vec![],
            date_provider: Box::new(|| -> NaiveDate { Utc::now().date_naive() }),
        }
    }

    #[cfg(test)]
    pub fn new_for_test(date_provider: Box<dyn Fn() -> NaiveDate + Send>) -> Self {
        Self {
            post_counter: Default::default(),
            history: vec![],
            date_provider,
        }
    }

    pub fn add(&mut self, post_name: &str, from: &str) {
        let cur_date = (self.date_provider)();

        if let Some(entry) = self.post_counter.get_mut(post_name) {
            if entry.total != 0 && entry.stats_date != cur_date {
                // Adding unique visits to history vector
                self.history.push(entry.clone());

                // And resetting entry
                entry.stats_date = cur_date;
                entry.total = 0;
                entry.origins.clear();
            }

            if !entry.origins.contains(from) {
                entry.origins.insert(from.to_string());
                entry.total += 1;
            }
        } else {
            let mut counter = PostCounter {
                post_id: post_name.to_string(),
                total: 1,
                origins: Default::default(),
                stats_date: Default::default(),
            };
            counter.origins.insert(from.to_string());
            counter.stats_date = cur_date;
            self.post_counter.insert(post_name.to_string(), counter);
        }
    }

    pub fn remove_history(&mut self) -> Option<Vec<PostCounter>> {
        let history = std::mem::take(&mut self.history);
        self.history.clear();
        Some(history)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    fn new_metrics(date_provider: Box<dyn Fn() -> NaiveDate + Send>) -> AccessMetrics {
        AccessMetrics {
            post_counter: Default::default(),
            history: vec![],
            date_provider,
        }
    }

    #[test]
    fn test_add() {
        let d = || NaiveDate::from_ymd_opt(2024, 05, 28).unwrap();
        let mut am = new_metrics(Box::new(d));
        am.add("post-1-name", "127.0.0.1");
        am.add("post-1-name", "10.1.2.3");
        am.add("post-2-name", "127.0.0.1");
        am.add("post-1-name", "127.0.0.1");
        assert_eq!(am.post_counter.len(), 2);
        assert_eq!(am.history.len(), 0);
        let post1 = am.post_counter.get("post-1-name").unwrap();
        let expected1 = PostCounter {
            post_id: "post-1-name".to_string(),
            total: 2,
            origins: vec!["127.0.0.1", "10.1.2.3"].iter().map(|x| x.to_string()).collect(),
            stats_date: NaiveDate::from_ymd_opt(2024, 05, 28).unwrap(),
        };
        assert_eq!(*post1, expected1);
        let post2 = am.post_counter.get("post-2-name").unwrap();
        let expected2 = PostCounter {
            post_id: "post-2-name".to_string(),
            total: 1,
            origins: vec!["127.0.0.1"].iter().map(|x| x.to_string()).collect(),
            stats_date: NaiveDate::from_ymd_opt(2024, 05, 28).unwrap(),
        };
        assert_eq!(*post2, expected2);
    }

    #[test]
    fn test_add_diff_date() {
        let d = || -> NaiveDate {
            static mut COUNTER: u32 = 0;
            let day = unsafe {
                COUNTER += 1;
                if COUNTER > 2 { 22 } else { 21 }
            };

            NaiveDate::from_ymd_opt(2024, 05, day).unwrap()
        };

        let mut am = new_metrics(Box::new(d));
        am.add("post-1-name", "10.1.2.3");
        am.add("post-1-name", "10.1.2.4");
        am.add("post-1-name", "10.1.2.5");

        assert_eq!(am.post_counter.len(), 1);
        assert_eq!(am.history.len(), 1);

        let post1 = am.post_counter.get("post-1-name").unwrap();
        let expected1 = PostCounter {
            post_id: "post-1-name".to_string(),
            total: 1,
            origins: vec!["10.1.2.5"].iter().map(|x| x.to_string()).collect(),
            stats_date: NaiveDate::from_ymd_opt(2024, 05, 22).unwrap(),
        };
        assert_eq!(*post1, expected1);

        let expected2 = vec![PostCounter {
            post_id: "post-1-name".to_string(),
            total: 2,
            origins: vec!["10.1.2.3", "10.1.2.4"].iter().map(|x| x.to_string()).collect(),
            stats_date: NaiveDate::from_ymd_opt(2024, 05, 21).unwrap(),
        }];
        assert_eq!(am.history, expected2);

        let expected_history = am.history.clone();
        let history = am.remove_history().unwrap();
        assert_eq!(history, expected_history);
    }

    type Callback = dyn Fn() + Send;

    #[test]
    fn test_123() {
        let cb: Box<Callback> = Box::new(|| println!("hello!"));
        thread::spawn(move || cb()).join().unwrap();
    }
}