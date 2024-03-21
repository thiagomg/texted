use std::collections::HashMap;
use std::string::ToString;

#[derive(PartialEq, Debug)]
pub struct QueryString {
    items: HashMap<String, String>,
}

impl QueryString {
    pub fn from(buf: &str) -> Self {
        let vs: Vec<(String, String)> = serde_urlencoded::from_str(buf).unwrap_or_else(|_| vec![]);
        let items: HashMap<String, String> = vs.into_iter().collect();

        QueryString {
            items,
        }
    }

    pub fn get_page(&self) -> u32 {
        let one = "1".to_string();
        let val = self.items.get("page").unwrap_or(&one);
        let val = val.parse().unwrap_or(1);
        if val <= 0 { return 1; }
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_page() {
        // QueryString {}
    }

    #[test]
    fn test_parse_query_str() {
        let buf = "bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter";
        let meal = vec![
            ("bread".to_owned(), "baguette".to_owned()),
            ("cheese".to_owned(), "comté".to_owned()),
            ("meat".to_owned(), "ham".to_owned()),
            ("fat".to_owned(), "butter".to_owned()),
        ].into_iter().collect::<HashMap<_, _>>();

        let expected = QueryString {
            items: meal,
        };

        assert_eq!(QueryString::from(buf), expected);
    }

    #[test]
    fn test_parse_invalid_query_str() {
        let buf = "";
        let expected = QueryString {
            items: Default::default(),
        };
        assert_eq!(QueryString::from(buf), expected);
    }

    #[test]
    fn test_parse_key_only_query_str() {
        let buf = "key-only";
        let expected: HashMap<String, String> = vec![("key-only", "")].iter().map(|(x, y)| (x.to_string(), y.to_string())).collect::<HashMap<_, _>>();
        assert_eq!(QueryString::from(buf), QueryString { items: expected });
    }
}