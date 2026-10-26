#![forbid(unsafe_code)]
//! Pure domain models, state-graph crawler, hardware log correlator, visual diffing,
//! local accessibility/geometry heuristics, and agent-ready report formatters.

pub mod config;
pub mod correlator;
pub mod crawler;
pub mod diff;
pub mod i18n;
pub mod model;
pub mod report;
pub mod vision_heuristics;

pub fn workspace_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!workspace_version().is_empty());
    }
}
