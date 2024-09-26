use config::Config;
use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

pub struct TracingGuard {
    _log_level: Level,
}

impl TracingGuard {
    pub fn init(config: &Config) -> anyhow::Result<Self> {
        let configuration: Configuration = config.get("tracing").unwrap_or_default();
        let log_level_filter = EnvFilter::builder()
            .parse_lossy(configuration.log_level.unwrap_or(Level::INFO.to_string()));
        let log_level =
            log_level_filter.max_level_hint().and_then(|f| f.into_level()).unwrap_or(Level::INFO);
        let log_format = configuration.log_format.unwrap_or("fmt".to_string());
        let log_format_layer = match log_format.as_str() {
            "json" => tracing_subscriber::fmt::layer().json().boxed(),
            "fmt" => tracing_subscriber::fmt::layer().boxed(),
            _ => tracing_subscriber::fmt::layer().boxed(),
        };

        tracing_subscriber::registry().with(log_level_filter).with(log_format_layer).init();

        let pkg_version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");
        let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");

        tracing::info! {
            event = "tracing_initialized",
            log_level = log_level.to_string(),
            pkg_version,
            git_hash
        };

        Ok(TracingGuard { _log_level: log_level })
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Configuration {
    #[serde(default)]
    log_level: Option<String>,
    #[serde(default)]
    log_format: Option<String>,
}
