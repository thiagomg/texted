pub struct Paginator<'a, T> {
    post_list: &'a Vec<T>,
    // (NaiveDateTime, String)
    page_size: u32,
    page_count: u32,
}

impl<'a, T> Paginator<'a, T> {
    pub fn from(post_list: &'a Vec<T>, page_size: u32) -> Self {
        if post_list.is_empty() {
            return Paginator {
                post_list,
                page_size,
                page_count: 0,
            };
        }
        let post_count = post_list.len() as u32;
        let upper_bound = post_count - 1;
        let page_count = (upper_bound / page_size) + 1;

        Paginator {
            post_list,
            page_size,
            page_count,
        }
    }

    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    pub fn get_page(&self, page: u32) -> Result<&[T], String> {
        match page {
            0 => return Err("Page has to be greater than 0".to_string()),
            x if x > self.page_count => return Err(format!("Page has to be less than page_count ({})", self.page_count)),
            _ => {}
        };

        let index = ((page - 1) * self.page_size) as usize;
        let mut page_size = (self.page_size as usize) + index;
        if page_size > self.post_list.len() {
            page_size = self.post_list.len();
        }
        let x = &self.post_list[index..page_size];
        Ok(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_case() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
        let paginator = Paginator::from(&items, 3);
        assert_eq!(paginator.page_count(), 5);
        assert_eq!(paginator.get_page(1), Ok(&[1, 2, 3].as_slice()).copied());
        assert_eq!(paginator.get_page(2), Ok(&[4, 5, 6].as_slice()).copied());
        assert_eq!(paginator.get_page(3), Ok(&[7, 8, 9].as_slice()).copied());
        assert_eq!(paginator.get_page(4), Ok(&[10, 11, 12].as_slice()).copied());
        assert_eq!(paginator.get_page(5), Ok(&[13].as_slice()).copied());

        assert_eq!(paginator.get_page(0), Err("Page has to be greater than 0".to_string()));
        assert_eq!(paginator.get_page(6), Err("Page has to be less than page_count (5)".to_string()));
    }

    #[test]
    fn test_empty() {
        let items: Vec<u32> = vec![];
        let paginator = Paginator::from(&items, 3);
        assert_eq!(paginator.page_count(), 0);
        assert_eq!(paginator.get_page(0), Err("Page has to be greater than 0".to_string()));
        assert_eq!(paginator.get_page(1), Err("Page has to be less than page_count (0)".to_string()));
    }
}
