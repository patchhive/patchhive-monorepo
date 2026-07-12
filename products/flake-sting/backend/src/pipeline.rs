mod analysis;
mod routes;

pub use routes::{
    auth_status, capabilities, gen_key, gen_service_token, health, history, history_detail, login,
    overview, rotate_service_token, runs, scan_github_actions, startup_checks_route,
};
