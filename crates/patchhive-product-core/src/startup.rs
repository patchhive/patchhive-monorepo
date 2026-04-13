use http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderName, HeaderValue, Method,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StartupCheckLevel {
    Ok,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StartupCheck {
    pub level: StartupCheckLevel,
    pub msg: String,
}

impl StartupCheck {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Ok,
            msg: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Info,
            msg: message.into(),
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Warn,
            msg: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Error,
            msg: message.into(),
        }
    }
}

pub fn count_errors(checks: &[StartupCheck]) -> usize {
    checks
        .iter()
        .filter(|check| check.level == StartupCheckLevel::Error)
        .count()
}

pub fn log_checks(checks: &[StartupCheck]) {
    for check in checks {
        match check.level {
            StartupCheckLevel::Error => tracing::error!("Config: {}", check.msg),
            StartupCheckLevel::Warn => tracing::warn!("Config: {}", check.msg),
            _ => tracing::info!("Config: {}", check.msg),
        }
    }
}

pub fn configured_port(port_env: &str, default_port: u16) -> u16 {
    std::env::var(port_env)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port > 0)
        .unwrap_or(default_port)
}

pub fn listen_addr(port_env: &str, default_port: u16) -> String {
    format!("0.0.0.0:{}", configured_port(port_env, default_port))
}

pub fn cors_layer() -> CorsLayer {
    let allow_any = std::env::var("PATCHHIVE_CORS_ORIGINS")
        .ok()
        .map(|value| value.split(',').any(|item| item.trim() == "*"))
        .unwrap_or(false);

    let mut allowed = std::env::var("PATCHHIVE_CORS_ORIGINS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty() && item != "*")
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    allowed.extend([
        "http://localhost:5173".to_string(),
        "http://localhost:5174".to_string(),
        "http://localhost:5175".to_string(),
        "http://localhost:5176".to_string(),
        "http://localhost:5177".to_string(),
        "http://localhost:5178".to_string(),
        "http://localhost:5179".to_string(),
        "http://localhost:5180".to_string(),
        "http://localhost:5181".to_string(),
        "http://127.0.0.1:5173".to_string(),
        "http://127.0.0.1:5174".to_string(),
        "http://127.0.0.1:5175".to_string(),
        "http://127.0.0.1:5176".to_string(),
        "http://127.0.0.1:5177".to_string(),
        "http://127.0.0.1:5178".to_string(),
        "http://127.0.0.1:5179".to_string(),
        "http://127.0.0.1:5180".to_string(),
        "http://127.0.0.1:5181".to_string(),
    ]);

    let allowed_values = allowed
        .into_iter()
        .filter_map(|origin| HeaderValue::from_str(&origin).ok())
        .collect::<Vec<_>>();

    let origin_layer = if allow_any {
        tracing::warn!("PATCHHIVE_CORS_ORIGINS includes '*' — allowing any browser origin");
        AllowOrigin::any()
    } else {
        AllowOrigin::predicate(move |origin: &HeaderValue, _request_parts| {
            allowed_values.iter().any(|allowed| allowed == origin)
                || origin
                    .to_str()
                    .map(|value| value.starts_with("http://localhost:") || value.starts_with("http://127.0.0.1:"))
                    .unwrap_or(false)
        })
    };

    CorsLayer::new()
        .allow_origin(origin_layer)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            ACCEPT,
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("x-api-key"),
        ])
}
