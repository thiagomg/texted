use std::io;
use std::io::ErrorKind;
use std::sync::Arc;

use ramhorns::Template;

use crate::content::Content;
use crate::text_utils::format_date_time;

#[derive(ramhorns::Content)]
struct ListPage<'a> {
    #[ramhorns(flatten)]
    route: &'a crate::routes::RouteContext,
    post_list: Vec<PostItem>,
    tags: Vec<ViewTag>,
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
struct ViewTag {
    tag: String,
    url: String,
}

#[derive(ramhorns::Content)]
struct ViewPagination {
    current: bool,
    number: u32,
    url: String,
}

pub struct ListRenderer<'a> {
    pub template: Template<'a>,
    pub page_size: u32,
}

impl ListRenderer<'_> {
    pub fn new(list_tpl_src: &str, page_size: u32) -> io::Result<ListRenderer<'_>> {
        let template = match Template::new(list_tpl_src) {
            Ok(x) => x,
            Err(e) => {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("Error parsing list template: {}", e),
                ));
            }
        };

        Ok(ListRenderer {
            template,
            page_size,
        })
    }

    pub fn render_route(
        &self,
        contents: &[Arc<Content>],
        cur_page: u32,
        tags: Vec<String>,
        source: &str,
        listing: &str,
        route: &crate::routes::RouteContext,
    ) -> String {
        let mut post_list = vec![];
        for content in contents {
            let (date, time) = format_date_time(&content.header.date);
            let post_item = PostItem {
                date,
                time,
                link: crate::routes::content_url(source, &content.link),
                title: content.title.clone(),
                summary: content.rendered.clone(),
            };
            post_list.push(post_item);
        }

        let mut page_list: Vec<ViewPagination> = Vec::with_capacity(self.page_size as usize);
        for i in 1..=self.page_size {
            let current = i == cur_page;
            page_list.push(ViewPagination {
                current,
                number: i,
                url: format!("?page={i}"),
            })
        }

        let tags: Vec<_> = tags
            .into_iter()
            .map(|tag| ViewTag {
                url: format!(
                    "/{}/{}/",
                    crate::routes::segment(listing),
                    crate::routes::segment(&tag)
                ),
                tag,
            })
            .collect();
        self.template.render(&ListPage {
            route,
            post_list,
            tags,
            page_list,
            show_pagination: self.page_size > 1,
        })
    }
}
