mod analysis;
mod routes;

pub use routes::{
    auth_status, capabilities, check_github_release, gen_key, gen_service_token, health, history,
    history_detail, login, overview, rotate_service_token, runs, startup_checks_route,
};
