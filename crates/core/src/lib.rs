#![forbid(unsafe_code)]
//! Pure domain models, state-graph crawler, hardware log correlator, visual diffing,
//! local accessibility/geometry heuristics, and agent-ready report formatters.

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
