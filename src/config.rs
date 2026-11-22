use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::{env, fs, io};

use serde::Deserialize;

use crate::util::toml_date::TomlDate;

#[derive(Deserialize)]
pub struct Paths {
    pub template_dir: PathBuf,
    pub public_dir: PathBuf,
    pub posts_dir: PathBuf,
    pub pages_dir: PathBuf,
}

#[derive(Deserialize, Clone)]
pub struct Comments {
    pub dir: PathBuf,
}

#[derive(Deserialize)]
pub struct Personal {
    pub activity_start_year: i32,
    pub blog_start_date: TomlDate,
}

#[derive(Deserialize)]
pub struct Defaults {
    pub index_base_name: Option<String>,
    pub summary_line_count: Option<i32>,
    pub summary_line_tag: Option<String>,
    pub page_size: u32,
    pub rendering_cache_enabled: bool,
}

#[derive(Deserialize)]
pub struct Server {
    pub address: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct Log {
    pub level: LogLevel,
    pub log_to_console: bool,
    pub location: Option<PathBuf>,
}

#[derive(Deserialize, Copy, Clone)]
pub enum LogLevel {
    Critical = 0,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Deserialize)]
pub struct Metrics {
    pub location: Option<PathBuf>,
    pub time_slot_secs: Option<i64>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ContentRouteConfig {
    pub directory: PathBuf,
    pub template: String,
    pub title: Option<String>,
    pub navigation: Option<Vec<String>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum RouteConfig {
    Page(ContentRouteConfig),
    Post(ContentRouteConfig),
    Postlist { source: String, template: String },
}

impl RouteConfig {
    pub fn content(&self) -> Option<&ContentRouteConfig> {
        match self {
            Self::Page(content) | Self::Post(content) => Some(content),
            Self::Postlist { .. } => None,
        }
    }

    pub fn template(&self) -> &str {
        match self {
            Self::Page(content) | Self::Post(content) => &content.template,
            Self::Postlist { template, .. } => template,
        }
    }
}

fn default_rss_source() -> String {
    "view".to_string()
}

#[derive(Deserialize)]
pub struct RssFeed {
    #[serde(default = "default_rss_source")]
    pub source: String,
    pub title: String,
    pub site_url: String,
    pub description: String,
    pub page_size: u32,
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
    pub personal: Personal,
    pub paths: Paths,
    pub defaults: Defaults,
    pub server: Server,
    pub log: Option<Log>,
    pub metrics: Option<Metrics>,
    pub rss_feed: Option<RssFeed>,
    pub comments: Option<Comments>,
}

fn parse_path(path: PathBuf) -> PathBuf {
    if path.starts_with("${exe_dir}") {
        let cur_exe = env::current_exe().unwrap();
        let exe_dir = cur_exe.parent().unwrap().to_str().unwrap();
        let str_path = path.to_str().unwrap();
        PathBuf::from(str_path.replace("${exe_dir}", exe_dir))
    } else {
        path
    }
}

pub fn read_config(cfg_path: &PathBuf) -> io::Result<Config> {
    let cfg_content = match fs::read_to_string(cfg_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(io::Error::new(
                e.kind(),
                format!(
                    "Error opening configuration file {}: {}",
                    cfg_path.to_str().unwrap(),
                    e
                ),
            ))
        }
    };

    let mut cfg: Config = match toml::from_str::<Config>(cfg_content.as_str()) {
        Ok(cfg) => cfg,
        Err(e) => {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Error parsing configuration file: {}", e),
            ))
        }
    };

    cfg.paths = Paths {
        template_dir: parse_path(cfg.paths.template_dir),
        public_dir: parse_path(cfg.paths.public_dir),
        posts_dir: parse_path(cfg.paths.posts_dir),
        pages_dir: parse_path(cfg.paths.pages_dir),
    };

    for route in cfg.routes.values_mut() {
        if let RouteConfig::Page(content) | RouteConfig::Post(content) = route {
            content.directory = parse_path(content.directory.clone());
        }
    }

    if let Some(ref mut comments) = cfg.comments {
        comments.dir = parse_path(comments.dir.clone());
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_routes_and_resolves_executable_relative_directories() {
        let root = std::env::temp_dir().join(format!("texted-config-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let file = root.join("texted.toml");
        fs::write(&file, format!("{}\n[routes.docs]\ntype = 'page'\ndirectory = '${{exe_dir}}/docs'\ntemplate = 'docs.tpl'\n[routes.contents]\ntype = 'postlist'\nsource = 'docs'\ntemplate = 'contents.tpl'\n", include_str!("../texted.toml"))).unwrap();
        let config = read_config(&file).unwrap();
        assert_eq!(config.rss_feed.as_ref().unwrap().source, "view");
        assert_eq!(
            config.routes["docs"].content().unwrap().directory,
            env::current_exe().unwrap().parent().unwrap().join("docs")
        );
        assert!(
            matches!(&config.routes["contents"], RouteConfig::Postlist { source, .. } if source == "docs")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
