use serde::{Deserialize, Serialize};

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
