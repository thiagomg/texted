use crate::metrics::metric_aggregator::Event;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct EventSlot {
    pub post_name: String,
    pub total: u64,
    pub origins: HashSet<String>,
    pub stats_date_start: DateTime<Utc>,
    pub stats_date_end: DateTime<Utc>,
}

impl EventSlot {
    pub fn from_event(event: Event, slot_size: &Duration) -> Self {
        let (stats_date_start, stats_date_end) = get_slot(&event.date_time, slot_size);
        let mut origins = HashSet::<String>::new();
        origins.insert(event.origin);

        EventSlot {
            post_name: event.post_name,
            total: event.total,
            origins,
            stats_date_start,
            stats_date_end,
        }
    }
}

/// Return start + end date/time
fn get_slot(date_time: &DateTime<Utc>, slot_size: &Duration) -> (DateTime<Utc>, DateTime<Utc>) {
    // Calculate the start of the time slot
    let slot_size_secs = slot_size.num_seconds();
    let timestamp_seconds = date_time.timestamp();
    let start_timestamp = timestamp_seconds - (timestamp_seconds % slot_size_secs);
    let start = DateTime::<Utc>::from_timestamp(start_timestamp, 0).unwrap();

    // Calculate the end of the time slot
    let end = start + *slot_size;

    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_5_second_slot() {
        let timestamp = Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 2).unwrap();
        let slot_size = Duration::seconds(5);
        let (start, end) = get_slot(&timestamp, &slot_size);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 5).unwrap());
    }

    #[test]
    fn test_10_second_slot() {
        let timestamp = Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 7).unwrap();
        let slot_size = Duration::seconds(10);
        let (start, end) = get_slot(&timestamp, &slot_size);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 10).unwrap());
    }

    #[test]
    fn test_15_second_slot() {
        let timestamp = Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 14).unwrap();
        let slot_size = Duration::seconds(15);
        let (start, end) = get_slot(&timestamp, &slot_size);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 15).unwrap());
    }

    #[test]
    fn test_30_second_slot() {
        let timestamp = Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 25).unwrap();
        let slot_size = Duration::seconds(30);
        let (start, end) = get_slot(&timestamp, &slot_size);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 30).unwrap());
    }

    #[test]
    fn test_60_second_slot() {
        let timestamp = Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 50).unwrap();
        let slot_size = Duration::seconds(60);
        let (start, end) = get_slot(&timestamp, &slot_size);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 11, 4, 9, 12, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2024, 11, 4, 9, 13, 0).unwrap());
    }
}