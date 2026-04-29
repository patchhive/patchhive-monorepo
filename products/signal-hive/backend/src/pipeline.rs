// pipeline.rs — SignalHive pipeline (modular)

mod analysis;
mod routes;
mod scanning;
mod scoring;
mod utils;

// Re-export route handlers for main.rs
pub use routes::{capabilities, history, history_detail, report, runs, scan, timeline};

// Re-export for schedule/scheduler endpoints in main.rs
pub use scanning::{run_schedule_now, start_scheduler};
