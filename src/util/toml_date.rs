use std::str::FromStr;

use chrono::{NaiveDate, ParseError};
use serde::Deserialize;

// Code adapted from https://www.seachess.net/notes/toml-dates/
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct TomlDate(pub NaiveDate);

impl<'de> Deserialize<'de> for TomlDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = toml::value::Datetime::deserialize(deserializer)?;
        let date = TomlDate::from_str(&value.to_string()).map_err(Error::custom)?;
        Ok(date)
    }
}

impl FromStr for TomlDate {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let naive = NaiveDate::from_str(s)?;
        Ok(Self(naive))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    pub struct Personal {
        pub activity_start_year: i32,
        pub blog_start_date: TomlDate,
    }

    #[derive(Deserialize)]
    pub struct Config {
        pub personal: Personal,
    }

    #[test]
    fn test_date_time() {
        let toml_str = r##"
[personal]
activity_start_year = 2000
blog_start_date = 2024-04-22
"##;
        let cfg: Config = toml::from_str::<Config>(toml_str).unwrap();
        assert_eq!(cfg.personal.blog_start_date, TomlDate(NaiveDate::from_ymd_opt(2024, 04, 22).unwrap()));
    }
}
