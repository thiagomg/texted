use crate::metrics::event_slot::EventSlot;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

pub struct Event {
    pub post_name: String,
    pub origin: String,
    pub date_time: DateTime<Utc>,
    pub total: u64,
}

pub struct MetricAggregator {
    slot_size: Duration,
    slots: HashMap<String, EventSlot>,
    history: Vec<EventSlot>,
}

impl MetricAggregator {
    pub fn new(slot_size: Duration) -> Self {
        Self {
            slot_size,
            slots: Default::default(),
            history: vec![],
        }
    }

    pub fn add(&mut self, post_name: &str, from: &str) {
        self.add_event(Event {
            post_name: post_name.to_string(),
            origin: from.to_string(),
            date_time: Utc::now(),
            total: 1,
        })
    }

    pub fn add_event(&mut self, event: Event) {
        if let Some(slot) = self.slots.get_mut(&event.post_name) {
            // We need to check if the event is inside the slot duration.
            if event.date_time < slot.stats_date_end {
                // If yes, add origin into the hashset and increase total
                let inserted = slot.origins.insert(event.origin);
                if inserted {
                    slot.total += event.total;
                }
                return;
            } else {
                // If not, add to history and reset
                let values: Vec<EventSlot> = self.slots.drain().map(|(_, v)| v).collect();
                self.history.extend(values);
            }
        }

        let post_name = event.post_name.clone();
        let slot = EventSlot::from_event(event, &self.slot_size);
        self.slots.insert(post_name, slot);
    }

    pub fn take_events(&mut self) -> Option<Vec<EventSlot>> {
        if self.history.is_empty() {
            return None;
        }

        Some(std::mem::take(&mut self.history))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashSet;

    fn create(post_no: i32, origin_no: i32, secs: u32, total: u64) -> Event {
        Event {
            post_name: format!("post-{}", post_no),
            origin: format!("10.0.0.{}", origin_no),
            date_time: Utc.with_ymd_and_hms(2024, 11, 01, 01, 02, secs).unwrap(),
            total,
        }
    }

    #[test]
    fn test_slots() {
        let mut m = MetricAggregator::new(Duration::seconds(5));
        assert_eq!(m.take_events(), None);
        m.add_event(create(1, 1, 0, 1));
        m.add_event(create(1, 1, 0, 1));
        m.add_event(create(1, 2, 1, 1));
        m.add_event(create(1, 1, 5, 1));
        let events = m.take_events();
        let expected = vec![EventSlot {
            post_name: "post-1".to_string(),
            total: 2,
            origins: HashSet::from(["10.0.0.1".to_string(), "10.0.0.2".to_string()]),
            stats_date_start: DateTime::parse_from_rfc3339("2024-11-01T01:02:00Z").unwrap().into(),
            stats_date_end: DateTime::parse_from_rfc3339("2024-11-01T01:02:05Z").unwrap().into(),
        }];
        assert_eq!(events.unwrap(), expected);

        m.add_event(create(1, 1, 10, 1));
        let events = m.take_events();
        let expected = vec![EventSlot {
            post_name: "post-1".to_string(),
            total: 1,
            origins: HashSet::from(["10.0.0.1".to_string()]),
            stats_date_start: DateTime::parse_from_rfc3339("2024-11-01T01:02:05Z").unwrap().into(),
            stats_date_end: DateTime::parse_from_rfc3339("2024-11-01T01:02:10Z").unwrap().into(),
        }];
        assert_eq!(events.unwrap(), expected);
        assert_eq!(m.take_events(), None);
    }
}