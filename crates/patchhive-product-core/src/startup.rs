use http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderName, HeaderValue, Method,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl StartupCheck {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Ok,
            msg: message.into(),
            code: None,
            status: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Info,
            msg: message.into(),
            code: None,
            status: None,
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Warn,
            msg: message.into(),
            code: None,
            status: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: StartupCheckLevel::Error,
            msg: message.into(),
            code: None,
            status: None,
        }
    }

    pub fn with_identity(mut self, code: impl Into<String>, status: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self.status = Some(status.into());
        self
    }
}

pub fn check_has_status(checks: &[StartupCheck], code: &str, status: &str) -> bool {
    checks
        .iter()
        .any(|check| check.code.as_deref() == Some(code) && check.status.as_deref() == Some(status))
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
    let bind_addr = std::env::var("PATCHHIVE_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
    format!("{bind_addr}:{}", configured_port(port_env, default_port))
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
        "http://localhost:5182".to_string(),
        "http://localhost:5183".to_string(),
        "http://127.0.0.1:5173".to_string(),
        "http://127.0.0.1:5174".to_string(),
        "http://127.0.0.1:5175".to_string(),
        "http://127.0.0.1:5176".to_string(),
        "http://127.0.0.1:5177".to_string(),
        "http://127.0.0.1:5178".to_string(),
        "http://127.0.0.1:5179".to_string(),
        "http://127.0.0.1:5180".to_string(),
        "http://127.0.0.1:5181".to_string(),
        "http://127.0.0.1:5182".to_string(),
        "http://127.0.0.1:5183".to_string(),
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
                || is_patchhive_local_dev_origin(origin)
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

fn is_patchhive_local_dev_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };

    let host_port = rest.split('/').next().unwrap_or(rest);
    let Some((host, port)) = split_origin_host_port(host_port) else {
        return false;
    };
    let Ok(port) = port.parse::<u16>() else {
        return false;
    };

    (5173..=5183).contains(&port) && is_local_dev_host(host)
}

fn split_origin_host_port(host_port: &str) -> Option<(&str, &str)> {
    if let Some(stripped) = host_port.strip_prefix('[') {
        let (host, rest) = stripped.split_once(']')?;
        let port = rest.strip_prefix(':')?;
        return Some((host, port));
    }

    host_port.rsplit_once(':')
}

fn is_local_dev_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(addr)) => addr.is_loopback() || addr.is_private() || addr.is_link_local(),
        Ok(IpAddr::V6(addr)) => addr.is_loopback(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_patchhive_local_dev_origin;
    use http::HeaderValue;

    #[test]
    fn allows_patchhive_frontend_on_lan_host() {
        let origin = HeaderValue::from_static("http://192.168.1.108:5183");
        assert!(is_patchhive_local_dev_origin(&origin));
    }

    #[test]
    fn rejects_non_patchhive_frontend_ports() {
        let origin = HeaderValue::from_static("http://192.168.1.108:3000");
        assert!(!is_patchhive_local_dev_origin(&origin));
    }

    #[test]
    fn rejects_public_hosts() {
        let origin = HeaderValue::from_static("https://example.com:5183");
        assert!(!is_patchhive_local_dev_origin(&origin));
    }
}
