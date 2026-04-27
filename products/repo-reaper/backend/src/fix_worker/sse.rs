// sse.rs — SSE event helpers for agent logging and status

use axum::response::sse::Event;
use chrono::Utc;
use serde_json::{json, Value};
use std::convert::Infallible;

use crate::state::AgentConfig;

pub fn sse_ev(event: &str, data: Value) -> Result<Event, Infallible> {
    Ok(Event::default().event(event).data(data.to_string()))
}

pub fn sse(event: &str, data: Value) -> Result<Event, Infallible> {
    sse_ev(event, data)
}

pub fn ts() -> String {
    Utc::now().format("%H:%M:%S").to_string()
}

pub fn alog(agent: &AgentConfig, msg: &str, kind: &str) -> Result<Event, Infallible> {
    sse_ev(
        "agent_log",
        json!({
            "agent_id": agent.id, "agent": agent.name, "role": agent.role,
            "msg": msg, "type": kind, "ts": ts()
        }),
    )
}

pub fn alog_raw(
    agent_id: &str,
    agent_name: &str,
    role: &str,
    msg: &str,
    kind: &str,
) -> Result<Event, Infallible> {
    sse_ev(
        "agent_log",
        json!({
            "agent_id": agent_id, "agent": agent_name, "role": role,
            "msg": msg, "type": kind, "ts": ts()
        }),
    )
}

pub fn astatus(agent_id: &str, status: &str, task: &str) -> Result<Event, Infallible> {
    sse_ev(
        "agent_status",
        json!({"agent_id": agent_id, "status": status, "task": task}),
    )
}
