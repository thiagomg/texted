use std::str::FromStr;

use chrono::{NaiveDate, ParseError};
use serde::Deserialize;

// Code adapted from https://www.seachess.net/notes/toml-dates/
#[derive(Copy, Clone, PartialEq)]
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
