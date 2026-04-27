// fix_worker.rs — RepoReaper fix worker (modular)

mod context;
mod memory;
mod orchestrate;
mod patch;
mod sse;
mod types;

// Re-export public API used by routes/pipeline
pub use orchestrate::fix_one;
pub use sse::{alog, astatus, sse};
pub use types::FixParams;
