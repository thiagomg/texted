//! Configurable content routes and shared navigation data.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{bail, Context, Result};
use chrono::NaiveDateTime;
use ramhorns::Template;
use spdlog::warn;

use crate::config::{Config, ContentRouteConfig, RouteConfig};
use crate::content::content_file::ContentFile;
use crate::content::content_format::ContentFormat;
use crate::content::content_renderer::{ContentRenderer, ImagePrefix, RenderOptions};
use crate::content::html_renderer::HtmlRenderer;
use crate::content::parsing_utils::{parse_texted_header, parse_title_html, parse_title_markdown};
use crate::content::texted_renderer::TextedRenderer;
use crate::content::Content;
use crate::content_cache::{ContentCache, Expire};
use crate::post_list::PostListType;
use crate::post_processor::{get_preview_option, list_post_files};

#[derive(Clone, Default, ramhorns::Content)]
pub struct NavigationItem {
    pub title: String,
    pub url: String,
    pub current: bool,
}

#[derive(Default, ramhorns::Content)]
pub struct RouteContext {
    pub route_name: String,
    pub route_title: String,
    pub route_url: String,
    pub navigation_pages: Vec<NavigationItem>,
    pub previous_page: Option<NavigationItem>,
    pub next_page: Option<NavigationItem>,
    pub content_url: String,
    pub comment_action: String,
    pub listing_url: String,
}

struct NavigationMetadata {
    title: String,
    date: NaiveDateTime,
}

impl NavigationMetadata {
    fn read(slug: &str, path: &Path) -> io::Result<Self> {
        let file = ContentFile::from_file(slug.into(), path.to_path_buf())?;
        let lines = file.raw_content.lines();
        let (header, title) = match file.format {
            ContentFormat::Texted => {
                let (header, lines, first) =
                    TextedRenderer::parse_markdown_header(&file.file_path, lines)?;
                (header, parse_title_markdown(lines, first).0)
            }
            ContentFormat::Html => {
                let (header, lines, first) = parse_texted_header(path, lines)?;
                (header, parse_title_html(lines, first).0)
            }
        };
        Ok(Self {
            title,
            date: header.date,
        })
    }
}

pub struct Routes {
    pub definitions: HashMap<String, RouteConfig>,
    pub files: HashMap<String, HashMap<String, PathBuf>>,
    content_cache: RwLock<ContentCache<Content>>,
    preview_cache: RwLock<ContentCache<Content>>,
    navigation_cache: RwLock<ContentCache<NavigationMetadata>>,
}

impl Routes {
    pub fn new(config: &Config) -> Result<Self> {
        let content = |directory, template: &str| ContentRouteConfig {
            directory,
            template: template.into(),
            title: None,
            navigation: None,
        };
        let mut definitions = HashMap::from([
            (
                "page".into(),
                RouteConfig::Page(content(config.paths.pages_dir.clone(), "page.tpl")),
            ),
            (
                "view".into(),
                RouteConfig::Post(content(config.paths.posts_dir.clone(), "view.tpl")),
            ),
            (
                "list".into(),
                RouteConfig::Postlist {
                    source: "view".into(),
                    template: "postlist.tpl".into(),
                },
            ),
        ]);
        definitions.extend(config.routes.clone());
        if config.defaults.page_size == 0 {
            bail!("defaults.page_size must be greater than zero");
        }
        let mut files = HashMap::new();
        let discovery = config
            .defaults
            .index_base_name
            .as_ref()
            .map(|name| PostListType::IndexBaseName(name.clone()))
            .unwrap_or(PostListType::AnyContentFile);
        for (name, route) in &definitions {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || matches!(name.as_str(), "public" | "rss")
            {
                bail!("Invalid or reserved route prefix: {name}");
            }
            let template = config.paths.template_dir.join(route.template());
            let text = fs::read_to_string(&template).with_context(|| {
                format!("Route {name}: cannot read template {}", template.display())
            })?;
            Template::new(text).with_context(|| format!("Route {name}: invalid template"))?;
            if let Some(content) = route.content() {
                let entries =
                    list_post_files(&content.directory, &discovery).with_context(|| {
                        format!(
                            "Route {name}: cannot discover content in {}",
                            content.directory.display()
                        )
                    })?;
                let mut links = HashMap::new();
                for entry in entries {
                    if !safe_segment(&entry.post_name) {
                        bail!("Route {name}: invalid content slug {}", entry.post_name);
                    }
                    if links
                        .insert(entry.post_name.clone(), entry.post_path)
                        .is_some()
                        && config.routes.contains_key(name)
                    {
                        bail!("Route {name}: duplicate content slug {}", entry.post_name);
                    }
                }
                if let Some(navigation) = &content.navigation {
                    let mut seen = HashSet::new();
                    for slug in navigation {
                        if !links.contains_key(slug) || !seen.insert(slug) {
                            bail!("Route {name}: missing or duplicate navigation entry {slug}");
                        }
                    }
                }
                files.insert(name.clone(), links);
            }
        }
        for (name, route) in &definitions {
            if let RouteConfig::Postlist { source, .. } = route {
                if !files.contains_key(source) {
                    bail!("Route {name}: source {source} must reference a page or post route");
                }
            }
        }
        if let Some(rss) = &config.rss_feed {
            if rss.page_size == 0 {
                bail!("rss_feed.page_size must be greater than zero");
            }
            if !matches!(definitions.get(&rss.source), Some(RouteConfig::Post(_))) {
                bail!("RSS source {} must reference a post route", rss.source);
            }
        }
        let cache = || {
            if config.defaults.rendering_cache_enabled {
                ContentCache::new()
            } else {
                ContentCache::non_caching()
            }
        };
        Ok(Self {
            definitions,
            files,
            content_cache: RwLock::new(cache()),
            preview_cache: RwLock::new(cache()),
            navigation_cache: RwLock::new(if config.defaults.rendering_cache_enabled {
                ContentCache::new()
            } else {
                ContentCache::non_caching()
            }),
        })
    }

    pub fn load(
        &self,
        config: &Config,
        prefix: &str,
        slug: &str,
        preview: bool,
    ) -> io::Result<Arc<Content>> {
        let cache = if preview {
            &self.preview_cache
        } else {
            &self.content_cache
        };
        let key = format!("{prefix}/{slug}");
        if let Some(content) = cache.read().unwrap().get_post(&key) {
            return Ok(content);
        }
        let path = self
            .files
            .get(prefix)
            .and_then(|links| links.get(slug))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Content not found"))?;
        let file = ContentFile::from_file(slug.into(), path.clone())?;
        let options = if preview {
            RenderOptions::PreviewOnly(
                get_preview_option(config),
                ImagePrefix(format!("/{}/{}", segment(prefix), segment(slug))),
            )
        } else {
            RenderOptions::FullContent
        };
        let content = match file.format {
            ContentFormat::Texted => TextedRenderer::render(&file, options),
            ContentFormat::Html => HtmlRenderer::render(&file, options),
        }?;
        Ok(cache
            .write()
            .unwrap()
            .add_post(&key, content, Expire::Never))
    }

    pub fn ordered(
        &self,
        config: &Config,
        prefix: &str,
        preview: bool,
    ) -> io::Result<Vec<Arc<Content>>> {
        let route = &self.definitions[prefix];
        let settings = route
            .content()
            .expect("content source validated at startup");
        if let Some(order) = &settings.navigation {
            return order
                .iter()
                .map(|slug| self.load(config, prefix, slug, preview))
                .collect();
        }
        let mut contents = self.files[prefix]
            .keys()
            .map(|slug| self.load(config, prefix, slug, preview))
            .collect::<io::Result<Vec<_>>>()?;
        contents.sort_by(|a, b| {
            let primary = if matches!(route, RouteConfig::Post(_)) {
                b.header.date.cmp(&a.header.date)
            } else {
                a.title.cmp(&b.title)
            };
            primary.then_with(|| a.link.cmp(&b.link))
        });
        Ok(contents)
    }

    fn navigation_metadata(
        &self,
        prefix: &str,
        slug: &str,
        path: &Path,
    ) -> io::Result<Arc<NavigationMetadata>> {
        let key = format!("{prefix}/{slug}");
        if let Some(metadata) = self.navigation_cache.read().unwrap().get_post(&key) {
            return Ok(metadata);
        }
        let metadata = NavigationMetadata::read(slug, path)?;
        Ok(self
            .navigation_cache
            .write()
            .unwrap()
            .add_post(&key, metadata, Expire::Never))
    }

    pub fn context(&self, prefix: &str, current: Option<&str>) -> RouteContext {
        let route = &self.definitions[prefix];
        let settings = route.content().unwrap();
        let files = &self.files[prefix];
        let slugs: Vec<_> = match &settings.navigation {
            Some(order) => order.iter().collect(),
            None => files.keys().collect(),
        };
        let mut entries: Vec<_> = slugs
            .into_iter()
            .filter_map(
                |slug| match self.navigation_metadata(prefix, slug, &files[slug]) {
                    Ok(metadata) => Some((slug, metadata)),
                    Err(error) => {
                        warn!("Skipping navigation entry {}/{}: {}", prefix, slug, error);
                        None
                    }
                },
            )
            .collect();
        if settings.navigation.is_none() {
            entries.sort_by(|(a_slug, a), (b_slug, b)| {
                let primary = if matches!(route, RouteConfig::Post(_)) {
                    b.date.cmp(&a.date)
                } else {
                    a.title.cmp(&b.title)
                };
                primary.then_with(|| a_slug.cmp(b_slug))
            });
        }
        let pages: Vec<_> = entries
            .into_iter()
            .map(|(slug, metadata)| NavigationItem {
                title: metadata.title.clone(),
                url: content_url(prefix, slug),
                current: current == Some(slug.as_str()),
            })
            .collect();
        let position = pages.iter().position(|page| page.current);
        let previous_page = position
            .and_then(|i| i.checked_sub(1))
            .map(|i| pages[i].clone());
        let next_page = position.and_then(|i| pages.get(i + 1)).cloned();
        let content_url = current
            .map(|slug| content_url(prefix, slug))
            .unwrap_or_default();
        let mut listings: Vec<_> = self
            .definitions
            .iter()
            .filter_map(|(name, route)| match route {
                RouteConfig::Postlist { source, .. } if source == prefix => Some(name.as_str()),
                _ => None,
            })
            .collect();
        listings.sort();
        if prefix == "view" && listings.contains(&"list") {
            listings.retain(|name| *name != "list");
            listings.insert(0, "list");
        }
        RouteContext {
            route_name: prefix.into(),
            route_title: settings.title.clone().unwrap_or_else(|| prefix.into()),
            route_url: format!("/{}/", segment(prefix)),
            navigation_pages: pages,
            previous_page,
            next_page,
            comment_action: if current.is_some() {
                format!("{content_url}comment")
            } else {
                String::new()
            },
            content_url,
            listing_url: listings
                .first()
                .map(|name| format!("/{}/", segment(name)))
                .unwrap_or_default(),
        }
    }

    pub fn attachment(&self, prefix: &str, slug: &str, file: &str) -> io::Result<PathBuf> {
        if !safe_segment(slug) || !safe_segment(file) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Invalid attachment path",
            ));
        }
        let source = self
            .files
            .get(prefix)
            .and_then(|files| files.get(slug))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Content not found"))?;
        let directory = &self.definitions[prefix].content().unwrap().directory;
        // Only directory content owns local attachments; flat files do not expose neighbouring files.
        let parent = source.parent().unwrap();
        if parent == directory {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No attachment directory",
            ));
        }
        let root = directory.canonicalize()?;
        let parent = parent.canonicalize()?;
        let path = parent.join(file).canonicalize()?;
        if !parent.starts_with(root) || !path.starts_with(parent) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Attachment escapes content directory",
            ));
        }
        Ok(path)
    }
}

pub fn safe_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['/', '\\', '\0'])
        && !value.chars().any(char::is_control)
        && Path::new(value).components().count() == 1
        && matches!(
            Path::new(value).components().next(),
            Some(Component::Normal(_))
        )
}

pub fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

pub fn tag_url(listing_url: &str, tag: &str) -> String {
    // Browsers normalize dot path segments, even when percent-encoded.
    if matches!(tag, "." | "..") {
        format!("{listing_url}?tag={}", segment(tag))
    } else {
        format!("{listing_url}{}/", segment(tag))
    }
}

pub fn content_url(prefix: &str, slug: &str) -> String {
    format!("/{}/{}/", segment(prefix), segment(slug))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::config::Comments;

    pub struct Fixture {
        pub root: PathBuf,
    }
    impl Fixture {
        pub fn new() -> Self {
            let root = std::env::temp_dir().join(format!("texted-routes-{}", uuid::Uuid::new_v4()));
            for directory in ["pages", "posts", "docs", "articles", "templates", "public"] {
                fs::create_dir_all(root.join(directory)).unwrap();
            }
            for template in [
                "page.tpl",
                "view.tpl",
                "postlist.tpl",
                "content.tpl",
                "list.tpl",
                "index.tpl",
            ] {
                fs::write(root.join("templates").join(template), match template {
                    "postlist.tpl" | "list.tpl" => "{{route_name}}|{{listing_url}}|{{#post_list}}{{title}}={{link}}:{{{summary}}};{{/post_list}}TAGS:{{#tags}}{{tag}}={{url}};{{/tags}}PAGES:{{#page_list}}{{number}}={{url}};{{/page_list}}",
                    "index.tpl" => "count={{post_count}}",
                    _ => "{{route_name}}|{{post_title}}|{{{post_content}}}|{{comment_action}}|COMMENTS={{comments_enabled}}:{{#comments}}{{author}}={{body_html}};{{/comments}}|NAV:{{#navigation_pages}}{{title}}={{url}}:{{current}};{{/navigation_pages}}|PREV:{{#previous_page}}{{title}}{{/previous_page}}|NEXT:{{#next_page}}{{title}}{{/next_page}}",
                }).unwrap();
            }
            let fixture = Self { root };
            fixture.write("pages/shared.md", "Legacy page", "page body", 1);
            fixture.write("posts/shared.md", "Legacy post", "legacy body", 1);
            fixture.write("docs/shared.md", "Alpha", "doc body", 1);
            fixture.write("docs/second.md", "Beta", "second body", 2);
            fixture.write("docs/hidden.md", "Hidden", "hidden body", 3);
            fixture.write("articles/shared.md", "Article", "article body", 1);
            fixture.write("articles/newer.md", "Newest", "newest body", 2);
            fixture
        }
        pub fn write(&self, file: &str, title: &str, body: &str, day: u32) {
            let path = self.root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("[ID]: # ({file})\n[DATE]: # (2024-01-{day:02} 12:00:00.000)\n[AUTHOR]: # (Ada)\n[TAGS]: # (rust web)\n\n# {title}\n\n{body}\n")).unwrap();
        }
        pub fn config(&self) -> Config {
            let mut config: Config = toml::from_str(include_str!("../texted.toml")).unwrap();
            config.paths.pages_dir = self.root.join("pages");
            config.paths.posts_dir = self.root.join("posts");
            config.paths.template_dir = self.root.join("templates");
            config.paths.public_dir = self.root.join("public");
            config.comments = Some(Comments {
                dir: self.root.join("comments"),
            });
            config.metrics = None;
            config.log = None;
            config.defaults.page_size = 2;
            config.routes = toml::from_str(&format!(
                r#"
[doc]
type = "page"
directory = "{}"
template = "content.tpl"
[article]
type = "post"
directory = "{}"
template = "content.tpl"
[tutorials]
type = "postlist"
source = "doc"
template = "list.tpl"
[blog]
type = "postlist"
source = "article"
template = "list.tpl"
[archive]
type = "postlist"
source = "article"
template = "postlist.tpl"
"#,
                self.root.join("docs").display(),
                self.root.join("articles").display()
            ))
            .unwrap();
            config
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn legacy_defaults_and_explicit_replacements() {
        let f = Fixture::new();
        let mut config = f.config();
        config.routes.clear();
        let routes = Routes::new(&config).unwrap();
        assert_eq!(routes.definitions.len(), 3);
        assert_eq!(
            routes.load(&config, "page", "shared", false).unwrap().title,
            "Legacy page"
        );
        config.routes.insert(
            "page".into(),
            RouteConfig::Page(ContentRouteConfig {
                directory: f.root.join("docs"),
                template: "content.tpl".into(),
                title: None,
                navigation: None,
            }),
        );
        let routes = Routes::new(&config).unwrap();
        assert_eq!(
            routes.load(&config, "page", "shared", false).unwrap().title,
            "Alpha"
        );
    }

    #[test]
    fn ordering_navigation_and_namespaced_caches() {
        let f = Fixture::new();
        let mut config = f.config();
        let routes = Routes::new(&config).unwrap();
        assert_eq!(
            routes
                .ordered(&config, "doc", false)
                .unwrap()
                .iter()
                .map(|c| c.link.as_str())
                .collect::<Vec<_>>(),
            ["shared", "second", "hidden"]
        );
        assert_eq!(
            routes.ordered(&config, "article", false).unwrap()[0].link,
            "newer"
        );
        assert_eq!(
            routes.load(&config, "doc", "shared", false).unwrap().title,
            "Alpha"
        );
        assert_eq!(
            routes
                .load(&config, "article", "shared", false)
                .unwrap()
                .title,
            "Article"
        );
        assert_eq!(
            routes.load(&config, "view", "shared", false).unwrap().title,
            "Legacy post"
        );
        if let RouteConfig::Page(content) = config.routes.get_mut("doc").unwrap() {
            content.navigation = Some(vec!["second".into(), "shared".into()]);
        }
        let routes = Routes::new(&config).unwrap();
        let first = routes.context("doc", Some("second"));
        assert!(first.previous_page.is_none());
        assert_eq!(first.next_page.unwrap().url, "/doc/shared/");
        let last = routes.context("doc", Some("shared"));
        assert_eq!(last.previous_page.unwrap().title, "Beta");
        assert!(last.next_page.is_none());
        assert!(last.navigation_pages[1].current);
        let hidden = routes.context("doc", Some("hidden"));
        assert!(hidden.previous_page.is_none() && hidden.next_page.is_none());
        assert_eq!(hidden.navigation_pages.len(), 2);
        assert!(routes.load(&config, "doc", "hidden", false).is_ok());
    }

    #[test]
    fn startup_rejects_invalid_definitions_and_content() {
        let f = Fixture::new();
        let invalid = [
            (
                "rss",
                "type = 'postlist'\nsource = 'doc'\ntemplate = 'list.tpl'",
            ),
            (
                "bad/path",
                "type = 'postlist'\nsource = 'doc'\ntemplate = 'list.tpl'",
            ),
            (
                "bad",
                "type = 'postlist'\nsource = 'missing'\ntemplate = 'list.tpl'",
            ),
            (
                "bad",
                "type = 'postlist'\nsource = 'blog'\ntemplate = 'list.tpl'",
            ),
            (
                "bad",
                "type = 'postlist'\nsource = 'doc'\ntemplate = 'missing.tpl'",
            ),
        ];
        for (prefix, definition) in invalid {
            let mut config = f.config();
            config
                .routes
                .insert(prefix.into(), toml::from_str(definition).unwrap());
            assert!(Routes::new(&config).is_err(), "{prefix}: {definition}");
        }
        for navigation in [
            vec!["missing".into()],
            vec!["shared".into(), "shared".into()],
        ] {
            let mut config = f.config();
            if let RouteConfig::Page(content) = config.routes.get_mut("doc").unwrap() {
                content.navigation = Some(navigation);
            }
            assert!(Routes::new(&config).is_err());
        }
        let mut config = f.config();
        config.rss_feed.as_mut().unwrap().source = "doc".into();
        assert!(Routes::new(&config).is_err());
        let mut config = f.config();
        config.defaults.page_size = 0;
        assert!(Routes::new(&config).is_err());
        let config = f.config();
        fs::write(f.root.join("templates/content.tpl"), "{{/unopened}}").unwrap();
        assert!(Routes::new(&config).is_err());
        fs::write(f.root.join("templates/content.tpl"), "{{post_title}}").unwrap();
        fs::remove_dir_all(f.root.join("articles")).unwrap();
        assert!(Routes::new(&config).is_err());
        fs::create_dir(f.root.join("articles")).unwrap();
        fs::create_dir(f.root.join("docs/shared")).unwrap();
        f.write("docs/shared/index.md", "Duplicate", "", 1);
        assert!(Routes::new(&config).is_err());
    }

    #[test]
    fn implicit_legacy_routes_keep_flat_file_precedence() {
        let f = Fixture::new();
        f.write(
            "pages/shared/index.md",
            "Directory page",
            "directory body",
            1,
        );
        f.write(
            "posts/shared/index.md",
            "Directory post",
            "directory body",
            1,
        );
        let config = f.config();
        let routes = Routes::new(&config).unwrap();
        for (prefix, expected) in [("page", "Legacy page"), ("view", "Legacy post")] {
            assert_eq!(
                routes.load(&config, prefix, "shared", false).unwrap().title,
                expected
            );
        }
    }

    #[test]
    fn uncached_edits_refresh_navigation_and_previews() {
        let f = Fixture::new();
        let mut config = f.config();
        config.defaults.rendering_cache_enabled = false;
        let routes = Routes::new(&config).unwrap();
        assert_eq!(
            routes.context("doc", None).navigation_pages[0].title,
            "Alpha"
        );
        f.write("docs/shared.md", "Updated", "updated body", 1);
        assert!(routes
            .context("doc", None)
            .navigation_pages
            .iter()
            .any(|p| p.title == "Updated"));
        assert!(routes
            .load(&config, "doc", "shared", true)
            .unwrap()
            .rendered
            .contains("updated body"));
    }

    #[test]
    fn attachments_and_preview_links_use_their_source() {
        let f = Fixture::new();
        f.write("docs/images/index.md", "Images", "![pic](pic.png)", 1);
        fs::write(f.root.join("docs/images/pic.png"), "picture").unwrap();
        let config = f.config();
        let routes = Routes::new(&config).unwrap();
        assert!(routes
            .load(&config, "doc", "images", true)
            .unwrap()
            .rendered
            .contains("/doc/images/pic.png"));
        assert_eq!(
            routes.attachment("doc", "images", "pic.png").unwrap(),
            f.root.join("docs/images/pic.png")
        );
        for file in ["..", "../shared.md", "/etc/passwd", "a\\b"] {
            assert!(routes.attachment("doc", "images", file).is_err());
        }
        assert!(routes.attachment("doc", "shared", "second.md").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                f.root.join("articles/shared.md"),
                f.root.join("docs/images/outside.md"),
            )
            .unwrap();
            assert!(routes.attachment("doc", "images", "outside.md").is_err());
        }
    }
}
