#![forbid(unsafe_code)]
//! Dual-backend Vision LLM client (Google Gemini Multimodal API and Local OpenAI-Compatible
//! Vision Endpoints) for grounded UI control detection, visual/UX critique, and goal planning.

pub fn ai_module_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_stub_works() {
        assert!(ai_module_ready());
    }
}
