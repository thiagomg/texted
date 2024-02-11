
pub fn parse_date_time(buf: &str) -> (&str, &str) {
    let v: Vec<&str> = match buf.split(' ').collect::<Vec<_>>() {
        v if v.len() == 1 => buf.split('_').collect(),
        v => v,
    };
    if v.len() == 2 {
        (v[0], v[1])
    } else {
        (buf, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_time() {
        let (date, time) = parse_date_time("2017-09-10 10:42:42.000");
        assert_eq!(date, "2017-09-10");
        assert_eq!(time, "10:42:42.000");

        let (date, time) = parse_date_time("2017-09-10_10:42:42.000");
        assert_eq!(date, "2017-09-10");
        assert_eq!(time, "10:42:42.000");
    }

    #[test]
    fn test_parse_date_only() {
        let (date, time) = parse_date_time("2017-09-10-sad");
        assert_eq!(date, "2017-09-10-sad");
        assert_eq!(time, "");
    }

    #[test]
    fn test_parse_date_empty() {
        let (date, time) = parse_date_time("");
        assert_eq!(date, "");
        assert_eq!(time, "");
    }
}