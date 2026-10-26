#![forbid(unsafe_code)]
//! Dual-backend Vision LLM client (`M2.3`) supporting Google Gemini Multimodal API
//! and Local OpenAI-Compatible Vision Endpoints (Ollama, LM Studio, vLLM), with automatic
//! graceful fallback to deterministic heuristics when offline or unconfigured.

use base64::Engine as _;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::time::Duration;
use ui_qa_core::config::{AiProviderConfig, QaGoalStep};
use ui_qa_core::model::{BoundingRect, Finding, FindingCategory, Severity, UiAction, UiElement};
use ui_qa_core::vision_heuristics::analyze_screen_heuristics;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCritiqueItem {
    pub severity: Severity,
    pub category: FindingCategory,
    pub title: String,
    pub description: String,
    pub suggested_fix: String,
    pub bounding_box: Option<[i32; 4]>, // [x, y, width, height]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiCritiqueResponseSchema {
    pub findings: Vec<AiCritiqueItem>,
}

pub struct UnifiedVisionClient {
    config: AiProviderConfig,
    http: reqwest::blocking::Client,
}

impl UnifiedVisionClient {
    pub fn new(config: AiProviderConfig) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { config, http }
    }

    pub fn config(&self) -> &AiProviderConfig {
        &self.config
    }

    /// Runs Layer 1 deterministic geometry/contrast checks AND Layer 2 Multimodal LLM UX critique.
    /// If the configured LLM is offline or no API key is set, gracefully returns Layer 1 findings
    /// plus rule-based UX improvement suggestions without failing the QA run.
    pub fn critique_screen(
        &self,
        step_index: usize,
        frame: &RgbaImage,
        elements: &[UiElement],
        recent_logs: &[String],
    ) -> Vec<Finding> {
        let mut findings = analyze_screen_heuristics(
            step_index,
            frame.width(),
            frame.height(),
            elements,
            Some(frame),
        );

        // Add deterministic Rust GUI UX improvement suggestions based on screen structure
        findings.extend(generate_structural_ux_suggestions(
            step_index,
            elements,
            recent_logs,
        ));

        match &self.config {
            AiProviderConfig::HeuristicsOnly => findings,
            AiProviderConfig::Gemini { model, api_key_env } => {
                if let Ok(api_key) = std::env::var(api_key_env) {
                    if !api_key.trim().is_empty() {
                        if let Ok(ai_items) =
                            self.call_gemini_critic(model, &api_key, frame, elements, recent_logs)
                        {
                            findings.extend(convert_ai_items(step_index, ai_items));
                        }
                    }
                }
                findings
            }
            AiProviderConfig::OpenAiCompatible {
                endpoint_url,
                model,
                api_key_env,
            } => {
                let api_key = api_key_env
                    .as_ref()
                    .and_then(|k| std::env::var(k).ok())
                    .unwrap_or_default();
                if let Ok(ai_items) = self.call_openai_compat_critic(
                    endpoint_url,
                    model,
                    &api_key,
                    frame,
                    elements,
                    recent_logs,
                ) {
                    findings.extend(convert_ai_items(step_index, ai_items));
                }
                findings
            }
        }
    }

    /// Selects the best UI action to fulfill a natural-language `QaGoalStep`.
    pub fn plan_goal_step(&self, goal: &QaGoalStep, elements: &[UiElement]) -> Option<UiAction> {
        let desc_lower = goal.description.to_ascii_lowercase();

        // Match interactive element whose name or role appears in the goal description
        for el in elements
            .iter()
            .filter(|e| e.is_enabled && e.supports_invoke)
        {
            let name_lower = el.name.to_ascii_lowercase();
            if !name_lower.is_empty() && desc_lower.contains(&name_lower) {
                let (cx, cy) = el.bounds.center();
                return Some(UiAction::Click {
                    element_id: el.id.clone(),
                    element_name: el.name.clone(),
                    x: cx,
                    y: cy,
                });
            }
        }

        // Fallback: pick first enabled interactive button
        elements
            .iter()
            .find(|e| e.is_enabled && e.supports_invoke)
            .map(|el| {
                let (cx, cy) = el.bounds.center();
                UiAction::Click {
                    element_id: el.id.clone(),
                    element_name: if el.name.is_empty() {
                        el.role.clone()
                    } else {
                        el.name.clone()
                    },
                    x: cx,
                    y: cy,
                }
            })
    }

    fn call_gemini_critic(
        &self,
        model: &str,
        api_key: &str,
        frame: &RgbaImage,
        elements: &[UiElement],
        recent_logs: &[String],
    ) -> Result<Vec<AiCritiqueItem>, String> {
        let b64_png = encode_png_base64(frame)?;
        let prompt = build_critic_prompt(frame.width(), frame.height(), elements, recent_logs);
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [
                    { "text": prompt },
                    {
                        "inlineData": {
                            "mimeType": "image/png",
                            "data": b64_png
                        }
                    }
                ]
            }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0.2
            }
        });

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Gemini HTTP {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        let text = json
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing text in Gemini response".to_string())?;

        parse_ai_critique_json(text)
    }

    fn call_openai_compat_critic(
        &self,
        endpoint_url: &str,
        model: &str,
        api_key: &str,
        frame: &RgbaImage,
        elements: &[UiElement],
        recent_logs: &[String],
    ) -> Result<Vec<AiCritiqueItem>, String> {
        let b64_png = encode_png_base64(frame)?;
        let prompt = build_critic_prompt(frame.width(), frame.height(), elements, recent_logs);
        let url = format!("{}/chat/completions", endpoint_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": model,
            "temperature": 0.2,
            "response_format": { "type": "json_object" },
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": prompt },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{b64_png}")
                        }
                    }
                ]
            }]
        });

        let mut req = self.http.post(&url).json(&body);
        if !api_key.is_empty() {
            req = req.bearer_auth(api_key);
        }

        let resp = req.send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("OpenAI-compat HTTP {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        let text = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing content in OpenAI-compat response".to_string())?;

        parse_ai_critique_json(text)
    }
}

pub fn encode_png_base64(frame: &RgbaImage) -> Result<String, String> {
    let mut buf = Cursor::new(Vec::new());
    frame
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}

fn build_critic_prompt(
    width: u32,
    height: u32,
    elements: &[UiElement],
    recent_logs: &[String],
) -> String {
    let el_json = serde_json::to_string(elements).unwrap_or_default();
    let logs_joined = recent_logs.join("\n");
    format!(
        "You are a senior Windows Rust GUI (`iced`, `egui`, `Slint`) and Hardware-in-the-Loop QA Architect.\n\
         Inspect this {width}x{height} application screenshot alongside its Windows UI Automation tree and hardware logs.\n\
         UIA Elements: {el_json}\n\
         Recent Hardware/App Logs:\n{logs_joined}\n\n\
         Return ONLY valid JSON matching:\n\
         {{\"findings\": [{{\"severity\": \"critical|high|medium|suggestion\", \"category\": \"visual_layout_bug|ux_friction|accessibility_gap|hardware_log_mismatch|improvement_suggestion\", \"title\": \"...\", \"description\": \"...\", \"suggested_fix\": \"...\", \"bounding_box\": [x, y, width, height]}}]}}"
    )
}

pub fn parse_ai_critique_json(raw: &str) -> Result<Vec<AiCritiqueItem>, String> {
    let trimmed = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed: AiCritiqueResponseSchema =
        serde_json::from_str(trimmed).map_err(|e| e.to_string())?;
    Ok(parsed.findings)
}

fn convert_ai_items(step_index: usize, items: Vec<AiCritiqueItem>) -> Vec<Finding> {
    items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| Finding {
            id: format!("AI-CRITIC-{step_index:03}-{idx:02}"),
            severity: item.severity,
            category: item.category,
            title: item.title,
            description: item.description,
            suggested_fix: item.suggested_fix,
            bounds: item
                .bounding_box
                .map(|b| BoundingRect::new(b[0], b[1], b[2].max(0) as u32, b[3].max(0) as u32)),
            repro_steps: vec![format!("AI Visual & UX Critique at step #{step_index}")],
            correlated_logs: Vec::new(),
            screenshot_path: None,
        })
        .collect()
}

fn generate_structural_ux_suggestions(
    step_index: usize,
    elements: &[UiElement],
    recent_logs: &[String],
) -> Vec<Finding> {
    let mut out = Vec::new();
    let interactive_count = elements.iter().filter(|e| e.supports_invoke).count();

    if interactive_count == 1 {
        if let Some(btn) = elements.iter().find(|e| e.supports_invoke) {
            if btn.bounds.width < 120 || btn.bounds.height < 32 {
                out.push(Finding {
                    id: format!("UX-TARGET-SIZE-{step_index:03}"),
                    severity: Severity::Suggestion,
                    category: FindingCategory::ImprovementSuggestion,
                    title: format!("Increase Click Target Padding on '{}'", btn.name),
                    description: format!(
                        "Primary action button '{}' is {}x{} px. Increasing minimum height to ≥36 px improves desktop touch/DPI usability.",
                        btn.name, btn.bounds.width, btn.bounds.height
                    ),
                    suggested_fix: "Set minimum button size (`egui::Button::new(...).min_size(egui::vec2(140.0, 36.0))` or `.padding([8, 16])` in `iced`).".to_string(),
                    bounds: Some(btn.bounds),
                    repro_steps: vec![format!("Observe primary button '{}' at step #{step_index}", btn.name)],
                    correlated_logs: recent_logs.to_vec(),
                    screenshot_path: None,
                });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_qa_core::model::ControlSource;

    #[test]
    fn parses_structured_ai_critique_json_and_plans_goal_step() {
        let sample_json = r#"```json
        {
          "findings": [
            {
              "severity": "high",
              "category": "visual_layout_bug",
              "title": "Baud Rate Dropdown Truncated",
              "description": "115200 bps text is clipped on the right edge.",
              "suggested_fix": "Set ComboBox width to 140px in egui.",
              "bounding_box": [40, 80, 90, 28]
            }
          ]
        }
        ```"#;

        let items = parse_ai_critique_json(sample_json).expect("parse json");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].severity, Severity::High);
        assert_eq!(items[0].title, "Baud Rate Dropdown Truncated");

        let client = UnifiedVisionClient::new(AiProviderConfig::HeuristicsOnly);
        let elements = vec![UiElement {
            id: "btn_cal".to_string(),
            name: "Calibrate Sensor".to_string(),
            role: "Button".to_string(),
            control_type_id: 50000,
            value: None,
            is_enabled: true,
            supports_invoke: true,
            bounds: BoundingRect::new(40, 60, 100, 28),
            source: ControlSource::UiaAccessKit,
        }];

        let goal = QaGoalStep {
            description: "Click Calibrate Sensor and verify hardware response".to_string(),
            expected_ui_text: None,
            expected_log_regex: Some("ACK".to_string()),
        };

        let action = client.plan_goal_step(&goal, &elements).expect("plan step");
        match action {
            UiAction::Click { element_name, .. } => assert_eq!(element_name, "Calibrate Sensor"),
            _ => panic!("expected Click"),
        }
    }
}
