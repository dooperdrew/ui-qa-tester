//! Fast local deterministic visual & accessibility heuristics (`M1.3`).
//! Detects overlapping sibling controls, out-of-bounds/clipped controls, unnamed interactive
//! widgets (missing AccessKit labels), and low WCAG luminance contrast without any LLM call.

use crate::model::{BoundingRect, Finding, FindingCategory, Severity, UiElement};
use image::RgbaImage;

/// Runs all deterministic geometry, AccessKit labeling, and contrast checks on a screen's elements.
pub fn analyze_screen_heuristics(
    step_index: usize,
    window_width: u32,
    window_height: u32,
    elements: &[UiElement],
    frame: Option<&RgbaImage>,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // 1. Check each control for missing accessibility name or out-of-window clipping
    for (idx, el) in elements.iter().enumerate() {
        let is_interactive = matches!(
            el.control_type_id,
            50000 | 50002 | 50003 | 50004 | 50005 | 50011 | 50013 | 50015 | 50019
        ) || el.supports_invoke;

        if is_interactive && el.name.trim().is_empty() {
            findings.push(Finding {
                id: format!("A11Y-NAME-{step_index:03}-{idx:02}"),
                severity: Severity::Medium,
                category: FindingCategory::AccessibilityGap,
                title: format!(
                    "Interactive {} Control Missing AccessKit / UIA Label",
                    el.role
                ),
                description: format!(
                    "Interactive control '{}' (type {}) at ({}, {}, {}x{}) has an empty accessible name, making screen-reader and deterministic UIA automation fragile.",
                    el.id, el.control_type_id, el.bounds.x, el.bounds.y, el.bounds.width, el.bounds.height
                ),
                suggested_fix: "Add an explicit accessible label in your Rust GUI widget (e.g. `.on_hover_text(...)` / AccessKit node label in `egui`, `Accessible` role/label in `Slint`, or text label in `iced`).".to_string(),
                bounds: Some(el.bounds),
                repro_steps: vec![format!("Inspect screen at step #{step_index}")],
                correlated_logs: Vec::new(),
                screenshot_path: None,
            });
        }

        // Check if control extends beyond the window dimensions (clipped widget)
        if el.bounds.width > 0
            && el.bounds.height > 0
            && window_width > 0
            && window_height > 0
            && (el.bounds.x < 0
                || el.bounds.y < 0
                || (el.bounds.x + el.bounds.width as i32) > (window_width as i32 + 2)
                || (el.bounds.y + el.bounds.height as i32) > (window_height as i32 + 2))
        {
            findings.push(Finding {
                id: format!("VIS-CLIP-{step_index:03}-{idx:02}"),
                severity: Severity::High,
                category: FindingCategory::VisualLayoutBug,
                title: format!("Control '{}' Extends Outside Window Bounds (Clipped)", el.name),
                description: format!(
                    "Control '{}' bounds ({}, {}, {}x{}) overflow the target window size ({}x{}).",
                    el.name,
                    el.bounds.x,
                    el.bounds.y,
                    el.bounds.width,
                    el.bounds.height,
                    window_width,
                    window_height
                ),
                suggested_fix: "Wrap the container in a responsive layout (`egui::ScrollArea`, `iced::widget::scrollable`, or `Slint` `Flickable` / flexible `HorizontalLayout`) so controls never clip at this window size.".to_string(),
                bounds: Some(el.bounds),
                repro_steps: vec![format!("Observe window ({window_width}x{window_height}) at step #{step_index}")],
                correlated_logs: Vec::new(),
                screenshot_path: None,
            });
        }
    }

    // 2. Check for overlapping interactive controls
    let interactive: Vec<&UiElement> = elements
        .iter()
        .filter(|e| e.supports_invoke && e.bounds.area() > 0)
        .collect();

    for i in 0..interactive.len() {
        for j in (i + 1)..interactive.len() {
            let a = interactive[i];
            let b = interactive[j];
            let overlap = a.bounds.intersection_area(&b.bounds);
            let min_area = a.bounds.area().min(b.bounds.area()).max(1);
            let max_area = a.bounds.area().max(b.bounds.area());
            let is_parent_container = overlap == min_area && max_area > min_area * 2;
            // If two sibling interactive controls overlap by > 15% of the smaller control's area
            if !is_parent_container && overlap * 100 / min_area > 15 && a.bounds != b.bounds {
                findings.push(Finding {
                    id: format!("VIS-OVERLAP-{step_index:03}-{i:02}-{j:02}"),
                    severity: Severity::High,
                    category: FindingCategory::VisualLayoutBug,
                    title: format!(
                        "Overlapping Interactive Controls: '{}' and '{}'",
                        a.name, b.name
                    ),
                    description: format!(
                        "Interactive controls '{}' {:?} and '{}' {:?} overlap by {} px².",
                        a.name, a.bounds, b.name, b.bounds, overlap
                    ),
                    suggested_fix: "Increase spacing or use a non-overlapping layout container (`ui.horizontal` / `ui.vertical` in `egui`, `Row`/`Column` with `.spacing(...)` in `iced`).".to_string(),
                    bounds: Some(a.bounds),
                    repro_steps: vec![format!("Observe overlapping controls at step #{step_index}")],
                    correlated_logs: Vec::new(),
                    screenshot_path: None,
                });
            }
        }
    }

    // 3. Optional pixel contrast check inside interactive control bounding boxes
    if let Some(img) = frame {
        for (idx, el) in interactive.iter().enumerate() {
            if let Some(ratio) = estimate_box_contrast_ratio(img, &el.bounds) {
                if ratio < 2.2 {
                    findings.push(Finding {
                        id: format!("VIS-CONTRAST-{step_index:03}-{idx:02}"),
                        severity: Severity::Suggestion,
                        category: FindingCategory::UxFriction,
                        title: format!(
                            "Low Visual Contrast ({ratio:.2}:1) on Control '{}'",
                            el.name
                        ),
                        description: format!(
                            "Control '{}' has an estimated foreground/background luminance contrast ratio of {ratio:.2}:1 (WCAG AA recommends ≥ 4.5:1 for normal text).",
                            el.name
                        ),
                        suggested_fix: "Increase text/icon contrast against the button background color in your theme palette.".to_string(),
                        bounds: Some(el.bounds),
                        repro_steps: vec![format!("Inspect control '{}' at step #{step_index}", el.name)],
                        correlated_logs: Vec::new(),
                        screenshot_path: None,
                    });
                }
            }
        }
    }

    findings
}

/// Computes the WCAG relative luminance contrast ratio `(L_max + 0.05) / (L_min + 0.05)`
/// across the 5th and 95th percentile pixel luminances inside `rect`.
pub fn estimate_box_contrast_ratio(img: &RgbaImage, rect: &BoundingRect) -> Option<f64> {
    let x0 = rect.x.max(0) as u32;
    let y0 = rect.y.max(0) as u32;
    let x1 = ((rect.x + rect.width as i32).max(0) as u32).min(img.width());
    let y1 = ((rect.y + rect.height as i32).max(0) as u32).min(img.height());

    if x1 <= x0 + 2 || y1 <= y0 + 2 {
        return None;
    }

    let mut luminances = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x, y).0;
            let lum = relative_luminance(p[0], p[1], p[2]);
            luminances.push(lum);
        }
    }

    if luminances.len() < 16 {
        return None;
    }

    luminances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let l_min = luminances[luminances.len() * 2 / 100];
    let l_max = luminances[(luminances.len().saturating_sub(1) * 99) / 100];
    if (l_max - l_min).abs() < 1e-4 {
        return None;
    }
    Some((l_max + 0.05) / (l_min + 0.05))
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    fn channel(c: u8) -> f64 {
        let s = (c as f64) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ControlSource;

    #[test]
    fn detects_overlapping_and_clipped_and_unnamed_controls() {
        let elements = vec![
            UiElement {
                id: "btn1".to_string(),
                name: "".to_string(), // Missing accessible name!
                role: "Button".to_string(),
                control_type_id: 50000,
                value: None,
                is_enabled: true,
                supports_invoke: true,
                bounds: BoundingRect::new(20, 20, 100, 40),
                source: ControlSource::UiaAccessKit,
            },
            UiElement {
                id: "btn2".to_string(),
                name: "Connect".to_string(),
                role: "Button".to_string(),
                control_type_id: 50000,
                value: None,
                is_enabled: true,
                supports_invoke: true,
                bounds: BoundingRect::new(50, 25, 100, 40), // Overlaps with btn1!
                source: ControlSource::UiaAccessKit,
            },
            UiElement {
                id: "btn3".to_string(),
                name: "Overflow Button".to_string(),
                role: "Button".to_string(),
                control_type_id: 50000,
                value: None,
                is_enabled: true,
                supports_invoke: true,
                bounds: BoundingRect::new(580, 100, 120, 40), // Overflows 640px width!
                source: ControlSource::UiaAccessKit,
            },
        ];

        let findings = analyze_screen_heuristics(1, 640, 420, &elements, None);
        assert!(findings
            .iter()
            .any(|f| f.category == FindingCategory::AccessibilityGap));
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Extends Outside Window Bounds")));
        assert!(findings
            .iter()
            .any(|f| f.title.contains("Overlapping Interactive Controls")));
    }
}
