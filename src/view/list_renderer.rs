use std::io;
use std::io::ErrorKind;

use ramhorns::Template;

use crate::content::Content;
use crate::text_utils::format_date_time;

#[derive(ramhorns::Content)]
struct ListPage<'a> {
    post_list: Vec<PostItem>,
    tags: Vec<ViewTag<'a>>,
    page_list: Vec<ViewPagination>,
    show_pagination: bool,
}

#[derive(ramhorns::Content)]
struct PostItem {
    date: String,
    time: String,
    link: String,
    title: String,
    summary: String,
}

#[derive(ramhorns::Content)]
struct ViewTag<'a> {
    tag: &'a str,
}

#[derive(ramhorns::Content)]
struct ViewPagination {
    current: bool,
    number: u32,
}

pub struct ListRenderer<'a> {
    pub template: Template<'a>,
    pub page_size: u32,
}

impl ListRenderer<'_> {
    pub fn new(list_tpl_src: &str, page_size: u32) -> io::Result<ListRenderer> {
        let template = match Template::new(list_tpl_src) {
            Ok(x) => x,
            Err(e) => {
                return Err(io::Error::new(ErrorKind::InvalidInput, format!("Error parsing list template: {}", e)));
            }
        };

        Ok(ListRenderer {
            template,
            page_size,
        })
    }

    pub fn render(&self, contents: &[Content], cur_page: u32, tags: Vec<String>) -> String {
        let mut post_list = vec![];
        for content in contents {
            let (date, time) = format_date_time(&content.header.date);
            let post_item = PostItem {
                date,
                time,
                link: format!("/view/{}", &content.link),
                title: content.title.clone(),
                summary: content.rendered.clone(),
            };
            post_list.push(post_item);
        }

        let mut page_list: Vec<ViewPagination> = Vec::with_capacity(self.page_size as usize);
        for i in 1..=self.page_size {
            let current = if i == cur_page { true } else { false };
            page_list.push(ViewPagination {
                current,
                number: i,
            })
        }

        let tags: Vec<_> = tags.iter().map(|t| ViewTag { tag: t.as_str() }).collect();
        let rendered = self.template.render(&ListPage {
            post_list,
            tags,
            page_list,
            show_pagination: true,
        });

        rendered
    }
}