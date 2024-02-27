use std::ops::Index;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;

fn to_int<T: std::str::FromStr>(num_str: &str, date_str: &str) -> Result<T, String> {
    match num_str.parse::<T>() {
        Ok(x) => Ok(x),
        Err(_) => Err(format!("Error parsing {} from the date {}", num_str, date_str)),
    }
}


pub fn parse_date_time(buf: &str) -> Result<NaiveDateTime, String> {
    let patt = r#"(\d{4})-(\d{0,2})-(\d{0,2}) (\d{0,2}):(\d{0,2}):(\d{0,2})(\.\d{0,3})?"#;
    let re = Regex::new(patt).unwrap();
    let Some(caps) = re.captures(buf) else {
        return Err(format!("Unable to parse date time {}", buf));
    };

    let to_i32 = |num_str: &str| to_int::<i32>(num_str, buf);
    let to_u32 = |num_str: &str| to_int::<u32>(num_str, buf);

    // We are using the regex approach to make it more flexible
    let y: i32 = to_i32(caps.index(1))?;
    let m: u32 = to_u32(caps.index(2))?;
    let d: u32 = to_u32(caps.index(3))?;
    let h: u32 = to_u32(caps.index(4))?;
    let mn: u32 = to_u32(caps.index(5))?;
    let s: u32 = to_u32(caps.index(6))?;

    let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
    let time = NaiveTime::from_hms_opt(h, mn, s).unwrap();

    let date_time = NaiveDateTime::new(
        date,
        time,
    );

    Ok(date_time)
}

pub fn format_date_time(date_time: &NaiveDateTime) -> (String, String) {
    let date = date_time.format("%Y-%m-%d").to_string();
    let time = date_time.format("%H:%M:%S").to_string();
    (date, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_time() {
        let date_time = parse_date_time("2017-09-10 10:42:32.123").unwrap();
        let (date, time) = format_date_time(&date_time);
        assert_eq!(date, "2017-09-10");
        assert_eq!(time, "10:42:32");

        let date_time = parse_date_time("2017-09-10 10:42:32").unwrap();
        let (date, time) = format_date_time(&date_time);
        assert_eq!(date, "2017-09-10");
        assert_eq!(time, "10:42:32");

        let date_time = parse_date_time("2017-09-10 10:42:32").unwrap();
        let (date, time) = format_date_time(&date_time);
        assert_eq!(date, "2017-09-10");
        assert_eq!(time, "10:42:32");
    }
}