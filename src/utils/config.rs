use std::{collections::BTreeMap, sync::OnceLock};

use log::LevelFilter;

use super::env;

const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

pub struct Config {
    // Logger configuration
    log_level: Vec<(String, LevelFilter)>,
    default_log_level: LevelFilter,
    max_log_level: LevelFilter,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn parse_level_filter(s: &str) -> Option<LevelFilter> {
    match s.trim().to_ascii_lowercase().as_str() {
        "off" => Some(LevelFilter::Off),
        "error" => Some(LevelFilter::Error),
        "warn" | "warning" => Some(LevelFilter::Warn),
        "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        _ => None,
    }
}

impl Config {
    fn parse_log_level() -> (Vec<(String, LevelFilter)>, LevelFilter, LevelFilter) {
        let mut map = BTreeMap::new();
        let mut max_log_level = DEFAULT_LOG_LEVEL;
        let mut default_log_level = DEFAULT_LOG_LEVEL;
        if let Ok(spec) = std::env::var("LOG_LEVEL") {
            for part in spec.split(',') {
                let mut kv = part.splitn(2, '=');

                // If no '=' is present, it's the default level
                if kv.clone().count() == 1 {
                    let value = kv.next().unwrap().trim().to_string();
                    if let Some(level) = parse_level_filter(&value) {
                        default_log_level = level;
                        if level > max_log_level {
                            max_log_level = level;
                        }
                    }
                } else {
                    let key = kv.next().unwrap().trim().to_string();
                    let value = kv.next().unwrap_or("info").trim().to_string();
                    if let Some(level) = parse_level_filter(&value) {
                        map.insert(key, level);
                        if level > max_log_level {
                            max_log_level = level;
                        }
                    }
                }
            }
        }

        (
            map.into_iter().map(|(k, v)| (k, v)).collect(),
            max_log_level,
            default_log_level,
        )
    }

    fn init() -> Self {
        env::load_env(); // Ensure env is loaded once
        let (log_level, max_log_level, default_log_level) = Self::parse_log_level();

        Config {
            log_level,
            max_log_level,
            default_log_level,
        }
    }

    pub fn get() -> &'static Self {
        CONFIG.get_or_init(|| Self::init())
    }

    /// Find the target's that start with `target` and return the most specific
    /// (longest prefix match) log level, or the default if none match.
    pub fn get_log_level(prefix: &str) -> LevelFilter {
        let data = &Self::get().log_level;

        for (key, level) in data.iter().rev() {
            if prefix.starts_with(key) {
                return *level;
            }
        }
        Self::get().default_log_level
    }

    pub fn get_max_log_level() -> LevelFilter {
        Self::get().max_log_level
    }
}
