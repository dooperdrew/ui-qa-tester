//! Autonomous UI Crawler State Machine (`M1.4`).
//! Builds a directed state graph $G = (V, E)$, filters out destructive controls,
//! selects the next unvisited interactive control, and flags dead/unresponsive buttons.

use crate::model::{Finding, FindingCategory, Severity, UiAction, UiElement};
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct AutonomousCrawler {
    destructive_regex: Regex,
    visited_states: HashSet<String>,
    executed_actions_per_state: HashMap<String, HashSet<String>>,
    dead_controls: Vec<Finding>,
}

impl AutonomousCrawler {
    pub fn new(destructive_pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            destructive_regex: Regex::new(destructive_pattern)?,
            visited_states: HashSet::new(),
            executed_actions_per_state: HashMap::new(),
            dead_controls: Vec::new(),
        })
    }

    /// Computes a deterministic structural hash for a screen given its UI elements and perceptual dHash.
    pub fn compute_state_hash(elements: &[UiElement], dhash: u64) -> String {
        let mut acc: u64 = dhash ^ 0x9E37_79B9_7F4A_7C15;
        for el in elements {
            for byte in el.name.as_bytes().iter().chain(el.role.as_bytes()) {
                acc = acc.wrapping_mul(1099511628211).wrapping_add(*byte as u64);
            }
            acc ^= (el.control_type_id as u64).wrapping_shl(16);
        }
        format!("state-{acc:016x}")
    }

    pub fn is_destructive_control(&self, element: &UiElement) -> bool {
        self.destructive_regex.is_match(&element.name)
            || self.destructive_regex.is_match(&element.id)
    }

    /// Selects the next safe, enabled, unvisited interactive control on the current screen.
    pub fn select_next_action(
        &mut self,
        state_hash: &str,
        elements: &[UiElement],
    ) -> Option<UiAction> {
        self.visited_states.insert(state_hash.to_string());
        let tried = self
            .executed_actions_per_state
            .entry(state_hash.to_string())
            .or_default();

        for el in elements {
            if !el.is_enabled || !el.supports_invoke {
                continue;
            }
            if self.destructive_regex.is_match(&el.name) || self.destructive_regex.is_match(&el.id)
            {
                continue;
            }
            let key = format!("{}:{}:{}", el.id, el.name, el.control_type_id);
            if tried.contains(&key) {
                continue;
            }

            tried.insert(key);
            let (cx, cy) = el.bounds.center();
            return Some(UiAction::Click {
                element_id: el.id.clone(),
                element_name: if el.name.is_empty() {
                    el.role.clone()
                } else {
                    el.name.clone()
                },
                x: cx,
                y: cy,
            });
        }

        None
    }

    /// Checks whether clicking an interactive control produced neither a visual/structural change
    /// nor any hardware/stdout log activity (indicating a dead/unwired UI button).
    pub fn evaluate_transition(
        &mut self,
        step_index: usize,
        prev_state_hash: &str,
        new_state_hash: &str,
        dhash_dist: u32,
        action: &UiAction,
        emitted_log_count: usize,
    ) -> Option<Finding> {
        if prev_state_hash == new_state_hash && dhash_dist == 0 && emitted_log_count == 0 {
            let label = match action {
                UiAction::Click { element_name, .. } => element_name.as_str(),
                UiAction::InputText { element_id, .. } => element_id.as_str(),
                UiAction::Toggle { element_name, .. } => element_name.as_str(),
            };
            let finding = Finding {
                id: format!("UX-DEAD-BTN-{step_index:03}"),
                severity: Severity::Medium,
                category: FindingCategory::UxFriction,
                title: format!(
                    "Unresponsive / Dead Control '{}': Zero UI or Hardware Response",
                    label
                ),
                description: format!(
                    "Actuating '{label}' at step #{step_index} produced zero visual pixel change (dHash distance = 0), zero UIA tree change, and zero stdout/stderr/hardware log lines."
                ),
                suggested_fix: "Verify the control's callback/message handler is wired in your Rust GUI state update function, or disable the control (`enabled(false)`) when the action is unavailable.".to_string(),
                bounds: None,
                repro_steps: vec![format!("Step #{step_index}: Actuate '{label}'")],
                correlated_logs: Vec::new(),
                screenshot_path: None,
            };
            self.dead_controls.push(finding.clone());
            return Some(finding);
        }
        None
    }

    pub fn visited_state_count(&self) -> usize {
        self.visited_states.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BoundingRect, ControlSource};

    #[test]
    fn crawler_skips_destructive_buttons_and_detects_dead_controls() {
        let mut crawler =
            AutonomousCrawler::new(r"(?i)\b(delete|factory\s*reset)\b").expect("regex");

        let elements = vec![
            UiElement {
                id: "btn_reset".to_string(),
                name: "Factory Reset Device".to_string(),
                role: "Button".to_string(),
                control_type_id: 50000,
                value: None,
                is_enabled: true,
                supports_invoke: true,
                bounds: BoundingRect::new(10, 10, 120, 36),
                source: ControlSource::UiaAccessKit,
            },
            UiElement {
                id: "btn_connect".to_string(),
                name: "Connect Sensor".to_string(),
                role: "Button".to_string(),
                control_type_id: 50000,
                value: None,
                is_enabled: true,
                supports_invoke: true,
                bounds: BoundingRect::new(10, 60, 120, 36),
                source: ControlSource::UiaAccessKit,
            },
        ];

        let state = AutonomousCrawler::compute_state_hash(&elements, 0x1234);
        let action = crawler
            .select_next_action(&state, &elements)
            .expect("should pick Connect Sensor");

        match &action {
            UiAction::Click { element_name, .. } => assert_eq!(element_name, "Connect Sensor"),
            _ => panic!("expected Click"),
        }

        // Next call on same state should return None because all non-destructive buttons were tried
        assert!(crawler.select_next_action(&state, &elements).is_none());

        // Evaluate dead transition
        let dead = crawler.evaluate_transition(1, &state, &state, 0, &action, 0);
        assert!(dead.is_some());
    }
}
