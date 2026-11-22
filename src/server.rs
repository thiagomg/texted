use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::comments::{
    comment_error_message, CaptchaStore, Comment, CommentStore, CommentsContext,
};
use crate::config::{Config, RouteConfig};
use crate::content_cache::{ContentCache, Expire};
use crate::metrics::metric_handler::MetricHandler;
use crate::metrics::metric_sender::MetricSender;
use crate::metrics::metric_writer::MetricWriter;
use crate::post_processor::{get_cur_page, render_index};
use crate::query_string::QueryString;
use crate::routes::{content_url, safe_segment, Routes};
use crate::util::toml_date::TomlDate;
use crate::view::{
    list_renderer::ListRenderer, post_renderer::PostRenderer, rss_renderer::RssChannel,
};
use anyhow::Result;
use chrono::Duration;
use ntex::web;
use ntex::web::HttpRequest;
use ntex_files::NamedFile;
use serde::Deserialize;
use spdlog::{info, warn};

struct AppState {
    routes: Routes,
    config: RwLock<Config>,
    post_cache: RwLock<ContentCache<String>>,
    metric_sender: MetricSender,
    comments: Option<CommentServices>,
}

struct CommentServices {
    stores: HashMap<String, CommentStore>,
    captcha: CaptchaStore,
}

#[derive(Deserialize)]
struct CommentForm {
    author: Option<String>,
    body: Option<String>,
    captcha_id: Option<String>,
    captcha_answer: Option<String>,
    website: Option<String>,
}

fn html(result: std::io::Result<String>) -> web::HttpResponse {
    match result {
        Ok(body) => web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(error) => {
            warn!("Error rendering content: {}", error);
            web::HttpResponse::InternalServerError().body("Error rendering content")
        }
    }
}

#[web::get("/{tail}*")]
async fn configured_route(
    req: HttpRequest,
    tail: web::types::Path<String>,
    app_state: web::types::State<Arc<AppState>>,
) -> web::HttpResponse {
    let tail = tail.into_inner();
    let parts: Vec<_> = tail.trim_end_matches('/').split('/').collect();
    if parts.is_empty() || !parts.iter().all(|part| safe_segment(part)) {
        return web::HttpResponse::NotFound().finish();
    }
    let prefix = parts[0];
    let Some(route) = app_state.routes.definitions.get(prefix) else {
        return web::HttpResponse::NotFound().finish();
    };
    let listing = matches!(route, RouteConfig::Postlist { .. });
    if (!listing && !(2..=3).contains(&parts.len())) || (listing && parts.len() > 2) {
        return web::HttpResponse::NotFound().finish();
    }
    if !listing && !app_state.routes.files[prefix].contains_key(parts[1]) {
        return web::HttpResponse::NotFound().finish();
    }
    if parts.len() == 3 {
        return match app_state
            .routes
            .attachment(prefix, parts[1], parts[2])
            .and_then(NamedFile::open)
        {
            Ok(file) => file.into_response(&req),
            Err(_) => web::HttpResponse::NotFound().finish(),
        };
    }
    if !req.path().ends_with('/') && !(prefix == "list" && listing && parts.len() == 1) {
        let mut location = format!("{}/", req.path());
        if let Some(query) = req.uri().query() {
            location.push('?');
            location.push_str(query);
        }
        return web::HttpResponse::TemporaryRedirect()
            .header("Location", location)
            .finish();
    }
    let origin = get_origin(&req);
    match route {
        RouteConfig::Postlist { .. } => {
            if prefix == "list" {
                app_state
                    .metric_sender
                    .list(parts.get(1).map(|tag| tag.to_string()), origin)
                    .await;
            } else {
                app_state.metric_sender.page(parts.join("/"), origin).await;
            }
        }
        RouteConfig::Post(_) => {
            app_state
                .metric_sender
                .view(
                    if prefix == "view" {
                        parts[1].into()
                    } else {
                        parts.join("/")
                    },
                    origin,
                )
                .await;
        }
        RouteConfig::Page(_) => {
            app_state
                .metric_sender
                .page(
                    if prefix == "page" {
                        parts[1].into()
                    } else {
                        parts.join("/")
                    },
                    origin,
                )
                .await;
        }
    }
    let config = app_state.config.read().unwrap();
    if let RouteConfig::Postlist { source, template } = route {
        return html((|| {
            let mut contents = app_state.routes.ordered(&config, source, true)?;
            let mut frequencies = HashMap::<String, usize>::new();
            for tag in contents.iter().flat_map(|content| &content.header.tags) {
                *frequencies.entry(tag.clone()).or_default() += 1;
            }
            let mut ranked_tags: Vec<_> = frequencies.into_iter().collect();
            ranked_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let tags = ranked_tags.into_iter().map(|(tag, _)| tag).collect();
            if let Some(tag) = parts.get(1) {
                contents.retain(|c| c.header.tags.iter().any(|t| t == tag));
            }
            let size = config.defaults.page_size as usize;
            let paginator = crate::paginator::Paginator::from(&contents, size as u32);
            let count = paginator.page_count();
            let requested = get_cur_page(req.clone());
            let current = if requested > count { 1 } else { requested };
            let page_contents = if contents.is_empty() {
                &[][..]
            } else {
                paginator.get_page(current).map_err(std::io::Error::other)?
            };
            let mut context = app_state.routes.context(&config, source, None)?;
            context.listing_url = format!("/{prefix}/");
            let template = std::fs::read_to_string(config.paths.template_dir.join(template))?;
            let renderer = ListRenderer::new(&template, count)?;
            Ok(renderer.render_route(page_contents, current, tags, source, prefix, &context))
        })());
    }
    let slug = parts[1];
    let page_key = format!("route/{prefix}/{slug}");
    if matches!(route, RouteConfig::Page(_)) {
        if let Some(cached) = app_state.post_cache.read().unwrap().get_page(&page_key) {
            return web::HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(cached.to_string());
        }
    }
    html((|| {
        let content = app_state.routes.load(&config, prefix, slug, false)?;
        let context = app_state.routes.context(&config, prefix, Some(slug))?;
        let comments = if matches!(route, RouteConfig::Post(_)) {
            build_comments_context(&app_state, prefix, slug, &req)
        } else {
            CommentsContext::disabled()
        };
        let template = std::fs::read_to_string(config.paths.template_dir.join(route.template()))?;
        let rendered = PostRenderer::new(&template)?.render_route(&content, &comments, &context);
        if matches!(route, RouteConfig::Page(_)) {
            app_state.post_cache.write().unwrap().add_page(
                &page_key,
                rendered.clone(),
                Expire::Never,
            );
        }
        Ok(rendered)
    })())
}

#[web::post("/{prefix}/{slug}/comment")]
async fn post_comment(
    path: web::types::Path<(String, String)>,
    form: web::types::Form<CommentForm>,
    app_state: web::types::State<Arc<AppState>>,
) -> web::HttpResponse {
    let (prefix, post_name) = path.into_inner();
    let Some(services) = app_state.comments.as_ref() else {
        return web::HttpResponse::NotFound().body("Comments are disabled");
    };
    let Some(store) = services.stores.get(&prefix) else {
        return web::HttpResponse::NotFound().finish();
    };
    if !app_state
        .routes
        .files
        .get(&prefix)
        .is_some_and(|files| files.contains_key(&post_name))
    {
        return web::HttpResponse::NotFound().body("Post not found");
    }
    if form
        .website
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
    {
        return comment_redirect(&prefix, &post_name, "comment_posted=1");
    }
    if !services.captcha.verify(
        form.captcha_id.as_deref().unwrap_or(""),
        form.captcha_answer.as_deref().unwrap_or(""),
    ) {
        return comment_redirect(&prefix, &post_name, "comment_error=captcha");
    }
    let comment = match Comment::new(
        form.author.clone().unwrap_or_default(),
        form.body.clone().unwrap_or_default(),
    ) {
        Ok(comment) => comment,
        Err(e) => {
            return comment_redirect(&prefix, &post_name, &format!("comment_error={}", e.code()))
        }
    };
    match store.append(&post_name, &comment) {
        Ok(()) => comment_redirect(&prefix, &post_name, "comment_posted=1"),
        Err(e) => {
            warn!("Error saving comment on {}/{}: {:?}", prefix, post_name, e);
            comment_redirect(&prefix, &post_name, &format!("comment_error={}", e.code()))
        }
    }
}

#[web::get("/rss")]
async fn rss(req: HttpRequest, app_state: web::types::State<Arc<AppState>>) -> web::HttpResponse {
    app_state.metric_sender.rss(get_origin(&req)).await;
    let config = app_state.config.read().unwrap();
    let Some(feed) = &config.rss_feed else {
        return web::HttpResponse::BadRequest().body("RSS feed is not available.");
    };
    let result = (|| -> Result<Vec<u8>> {
        let mut contents = app_state.routes.ordered(&config, &feed.source, true)?;
        contents.truncate(feed.page_size as usize);
        Ok(RssChannel {
            ch_title: &feed.title,
            ch_link: &feed.site_url,
            ch_desc: &feed.description,
        }
        .render_route(&contents, &feed.source)?)
    })();
    match result {
        Ok(xml) => web::HttpResponse::Ok()
            .content_type("application/rss+xml; charset=UTF-8")
            .body(xml),
        Err(e) => {
            warn!("Error rendering RSS: {}", e);
            web::HttpResponse::InternalServerError().finish()
        }
    }
}

#[web::get("/public/{file_name}")]
async fn public_files(
    path: web::types::Path<String>,
    app_state: web::types::State<Arc<AppState>>,
) -> Result<NamedFile, web::Error> {
    if !safe_segment(&path) {
        return Err(web::error::ErrorNotFound("File not found").into());
    }
    let config = app_state.config.read().unwrap();
    Ok(NamedFile::open(
        config.paths.public_dir.join(path.into_inner()),
    )?)
}

#[web::get("/")]
async fn index(req: HttpRequest, app_state: web::types::State<Arc<AppState>>) -> web::HttpResponse {
    let origin: String = get_origin(&req);
    app_state.metric_sender.index(origin).await;

    let read_cache = app_state.post_cache.read().unwrap();
    let page_name = "-index-page";

    let rendered_page = match read_cache.get_page(page_name) {
        None => {
            // Let's load from the file and update the cache
            info!("Rendering page {} from file", page_name);
            let config = app_state.config.read().unwrap();
            let num_of_posts = app_state
                .routes
                .files
                .get(
                    config
                        .rss_feed
                        .as_ref()
                        .map(|feed| feed.source.as_str())
                        .unwrap_or("view"),
                )
                .map_or(0, |files| files.len());

            let TomlDate(blog_start_date) = config.personal.blog_start_date;
            let activity_start_year = config.personal.activity_start_year;

            let rendered_post = match render_index(
                req,
                num_of_posts,
                &config.paths.template_dir,
                activity_start_year,
                blog_start_date,
            ) {
                Ok(rendered_post) => rendered_post,
                Err(e) => {
                    return web::HttpResponse::BadRequest()
                        .body(format!("Error loading index page {}: {}", page_name, e));
                }
            };

            drop(read_cache);

            let mut rw_cache = app_state.post_cache.write().unwrap();
            rw_cache.add_page(page_name, rendered_post, Expire::After(Duration::days(1)))
        }
        Some(rendered) => rendered,
    }
    .to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_page)
}

fn build_comments_context(
    app_state: &AppState,
    prefix: &str,
    post_name: &str,
    req: &HttpRequest,
) -> CommentsContext {
    let Some(services) = app_state.comments.as_ref() else {
        return CommentsContext::disabled();
    };

    let comments = match services.stores[prefix].load(post_name) {
        Ok(comments) => comments,
        Err(e) => {
            warn!("Error loading comments for {}: {}", post_name, e);
            vec![]
        }
    };
    let captcha = services.captcha.generate();
    let (error, posted) = comment_feedback(req);
    CommentsContext::from_comments(comments, captcha, error, posted)
}

fn comment_feedback(req: &HttpRequest) -> (String, bool) {
    let Some(query) = req.uri().query() else {
        return (String::new(), false);
    };
    let qs = QueryString::from(query);
    let posted = qs.get("comment_posted").map(|s| s.as_str()) == Some("1");
    let error = qs
        .get("comment_error")
        .map(|code| comment_error_message(code))
        .unwrap_or_default();
    (error, posted)
}

fn comment_redirect(prefix: &str, post_name: &str, query: &str) -> web::HttpResponse {
    web::HttpResponse::SeeOther()
        .header(
            "Location",
            format!("{}?{}#comments", content_url(prefix, post_name), query),
        )
        .finish()
}

fn get_origin(req: &web::HttpRequest) -> String {
    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(addr) = header.to_str() {
            return addr.to_string();
        }
    }

    req.peer_addr().map_or("".to_string(), |x| format!("{}", x))
}

fn make_state(
    config: Config,
    routes: Routes,
    metric_sender: MetricSender,
) -> Result<Arc<AppState>> {
    let comments = if let Some(settings) = &config.comments {
        let mut stores = HashMap::new();
        for (name, route) in &routes.definitions {
            if matches!(route, RouteConfig::Post(_)) {
                let directory = if name == "view" {
                    settings.dir.clone()
                } else {
                    settings.dir.join("routes").join(name)
                };
                stores.insert(name.clone(), CommentStore::new(directory)?);
            }
        }
        Some(CommentServices {
            stores,
            captcha: CaptchaStore::new(),
        })
    } else {
        None
    };
    let post_cache = if config.defaults.rendering_cache_enabled {
        ContentCache::new()
    } else {
        ContentCache::non_caching()
    };
    Ok(Arc::new(AppState {
        routes,
        config: RwLock::new(config),
        post_cache: RwLock::new(post_cache),
        metric_sender,
        comments,
    }))
}

pub async fn server_run(config: Config) -> Result<()> {
    let routes = Routes::new(&config)?;
    let (metric_sender, _metrics) = if let Some(ref metrics_cfg) = config.metrics {
        let location = metrics_cfg.location.as_ref().unwrap();
        let metrics = MetricHandler::new(MetricWriter::new(
            location,
            Duration::seconds(metrics_cfg.time_slot_secs.unwrap()),
        )?);
        (metrics.new_sender(), Some(metrics))
    } else {
        (MetricHandler::no_op(), None)
    };
    let address = config.server.address.clone();
    let port = config.server.port;
    let state = make_state(config, routes, metric_sender)?;
    web::HttpServer::new(move || {
        web::App::new()
            .state(state.clone())
            .service(index)
            .service(public_files)
            .service(rss)
            .service(post_comment)
            .service(configured_route)
    })
    .bind((address, port))?
    .run()
    .await
    .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::tests::Fixture;
    use ntex::http::StatusCode;
    use ntex::web::test;

    #[ntex::test]
    async fn configurable_routes_render_and_keep_legacy_urls() {
        let f = Fixture::new();
        let config = f.config();
        let routes = Routes::new(&config).unwrap();
        let state = make_state(config, routes, MetricHandler::no_op()).unwrap();
        let app = test::init_service(
            web::App::new()
                .state(state)
                .service(index)
                .service(public_files)
                .service(rss)
                .service(post_comment)
                .service(configured_route),
        )
        .await;
        for (url, expected) in [
            ("/page/shared/", "page|Legacy page"),
            ("/view/shared/", "view|Legacy post"),
            ("/doc/shared/", "doc|Alpha"),
            ("/article/shared/", "article|Article"),
            ("/tutorials/", "Alpha=/doc/shared/"),
            ("/blog/", "Newest=/article/newer/"),
            ("/archive/", "Newest=/article/newer/"),
            ("/list", "Legacy post=/view/shared/"),
            ("/list/", "Legacy post=/view/shared/"),
            ("/", "count=1"),
            ("/blog/rust/", "rust=/blog/rust/"),
            ("/tutorials/?page=2", "Hidden=/doc/hidden/"),
        ] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK, "{url}");
            let body = test::read_body(response).await;
            assert!(
                std::str::from_utf8(&body).unwrap().contains(expected),
                "{url}: {:?}",
                body
            );
        }
        for url in ["/doc/shared?q=1", "/blog?q=1", "/blog/rust?q=1"] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
            assert_eq!(
                response
                    .headers()
                    .get("location")
                    .unwrap()
                    .to_str()
                    .unwrap(),
                url.replace("?", "/?")
            );
        }
        for url in [
            "/unknown/",
            "/doc/missing/",
            "/doc/",
            "/blog/rust/file",
            "/public/missing",
            "/rss/missing/",
        ] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{url}");
        }
    }

    #[ntex::test]
    async fn curated_lists_empty_results_html_and_attachments() {
        let f = Fixture::new();
        let mut config = f.config();
        f.write("docs/images/index.md", "Images", "![pic](pic.png)", 1);
        std::fs::write(f.root.join("docs/images/pic.png"), "image data").unwrap();
        std::fs::write(f.root.join("docs/html.htm"), "<!--\n[ID]: # (html)\n[DATE]: # (2024-01-01 12:00:00.000)\n[AUTHOR]: # (Ada)\n-->\n<h1>HTML page</h1>\n<p>HTML body</p>").unwrap();
        if let RouteConfig::Page(content) = config.routes.get_mut("doc").unwrap() {
            content.navigation = Some(vec!["images".into(), "shared".into()]);
        }
        let routes = Routes::new(&config).unwrap();
        let state = make_state(config, routes, MetricHandler::no_op()).unwrap();
        let app = test::init_service(web::App::new().state(state).service(configured_route)).await;
        for (url, text) in [
            ("/tutorials/", "Images=/doc/images/"),
            ("/doc/html/", "HTML body"),
            ("/doc/hidden/", "Hidden"),
            ("/doc/images/pic.png", "image data"),
        ] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK, "{url}");
            let body = test::read_body(response).await;
            assert!(std::str::from_utf8(&body).unwrap().contains(text));
            if url == "/tutorials/" {
                let body = std::str::from_utf8(&body).unwrap();
                assert!(!body.contains("Hidden="));
                assert!(body.contains("/doc/images/pic.png"));
            }
        }
        let response = test::call_service(
            &app,
            test::TestRequest::with_uri("/tutorials/no-such-tag/").to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = test::read_body(response).await;
        assert!(!std::str::from_utf8(&body).unwrap().contains("Images="));
        for url in [
            "/doc/images/%2E%2E",
            "/doc/images/%2Fetc%2Fpasswd",
            "/doc/images/missing.png",
        ] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{url}");
        }
    }

    #[ntex::test]
    async fn comments_and_rss_follow_the_post_source() {
        let f = Fixture::new();
        let mut config = f.config();
        config.rss_feed.as_mut().unwrap().source = "article".into();
        let routes = Routes::new(&config).unwrap();
        let state = make_state(config, routes, MetricHandler::no_op()).unwrap();
        let services = state.comments.as_ref().unwrap();
        services.stores["view"]
            .append(
                "shared",
                &Comment::new("Legacy".into(), "Legacy comment".into()).unwrap(),
            )
            .unwrap();
        let challenge = services.captcha.generate();
        let tokens: Vec<_> = challenge
            .question
            .trim_end_matches('?')
            .split_whitespace()
            .collect();
        let a: i32 = tokens[2].parse().unwrap();
        let b: i32 = tokens[4].parse().unwrap();
        let answer = if tokens[3] == "+" { a + b } else { a - b }.to_string();
        let app = test::init_service(
            web::App::new()
                .state(state.clone())
                .service(index)
                .service(rss)
                .service(post_comment)
                .service(configured_route),
        )
        .await;
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/article/shared/comment")
                .set_form(&[
                    ("author", "Custom"),
                    ("body", "Custom comment"),
                    ("captcha_id", &challenge.id),
                    ("captcha_answer", &answer),
                ])
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/article/shared/?comment_posted=1#comments"
        );
        assert_eq!(
            services.stores["view"].load("shared").unwrap()[0].author,
            "Legacy"
        );
        assert_eq!(
            services.stores["article"].load("shared").unwrap()[0].author,
            "Custom"
        );
        assert!(f.root.join("comments/shared.jsonl").is_file());
        assert!(f
            .root
            .join("comments/routes/article/shared.jsonl")
            .is_file());
        for (url, expected, absent) in [
            (
                "/article/shared/",
                "Custom=Custom comment",
                "Legacy comment",
            ),
            ("/view/shared/", "Legacy=Legacy comment", "Custom comment"),
            ("/doc/shared/", "COMMENTS=false", "Custom comment"),
            ("/rss", "/article/newer/", "/view/"),
            ("/", "count=2", "count=1"),
        ] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK, "{url}");
            let body = test::read_body(response).await;
            let text = std::str::from_utf8(&body).unwrap();
            assert!(text.contains(expected), "{url}: {text}");
            assert!(!text.contains(absent), "{url}: {text}");
        }
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/doc/shared/comment")
                .set_form(&[("author", "Ada"), ("body", "Denied")])
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    #[ntex::test]
    async fn page_cache_setting_controls_edits_and_render_errors() {
        for enabled in [true, false] {
            let f = Fixture::new();
            let mut config = f.config();
            config.defaults.rendering_cache_enabled = enabled;
            let routes = Routes::new(&config).unwrap();
            let state = make_state(config, routes, MetricHandler::no_op()).unwrap();
            let app =
                test::init_service(web::App::new().state(state).service(configured_route)).await;
            let response = test::call_service(
                &app,
                test::TestRequest::with_uri("/doc/shared/").to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            f.write("docs/shared.md", "Updated", "edited body", 1);
            let response = test::call_service(
                &app,
                test::TestRequest::with_uri("/doc/shared/").to_request(),
            )
            .await;
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains(if enabled { "Alpha" } else { "Updated" }));
            std::fs::write(f.root.join("templates/content.tpl"), "{{/unopened}}").unwrap();
            let response = test::call_service(
                &app,
                test::TestRequest::with_uri("/doc/shared/").to_request(),
            )
            .await;
            assert_eq!(
                response.status(),
                if enabled {
                    StatusCode::OK
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            );
        }
    }

    #[ntex::test]
    async fn empty_sources_render_lists_and_rss() {
        let f = Fixture::new();
        let mut config = f.config();
        config.rss_feed.as_mut().unwrap().source = "article".into();
        std::fs::remove_file(f.root.join("articles/shared.md")).unwrap();
        std::fs::remove_file(f.root.join("articles/newer.md")).unwrap();
        let routes = Routes::new(&config).unwrap();
        let state = make_state(config, routes, MetricHandler::no_op()).unwrap();
        let app = test::init_service(
            web::App::new()
                .state(state)
                .service(rss)
                .service(configured_route),
        )
        .await;
        for url in ["/blog/", "/blog/?page=999", "/rss"] {
            let response =
                test::call_service(&app, test::TestRequest::with_uri(url).to_request()).await;
            assert_eq!(response.status(), StatusCode::OK, "{url}");
        }
    }
}
