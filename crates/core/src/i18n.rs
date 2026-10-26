//! Localization catalog loader (`locales/en.json`, `locales/it.json`).

use std::collections::HashMap;
use std::path::Path;

const EMBEDDED_EN: &str = include_str!("../../../locales/en.json");
const EMBEDDED_IT: &str = include_str!("../../../locales/it.json");

#[derive(Debug, Clone)]
pub struct I18nCatalog {
    locale: String,
    strings: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl I18nCatalog {
    pub fn new(locale: &str) -> Self {
        let fallback: HashMap<String, String> =
            serde_json::from_str(EMBEDDED_EN).unwrap_or_default();
        let strings = match locale {
            "it" => serde_json::from_str(EMBEDDED_IT).unwrap_or_else(|_| fallback.clone()),
            _ => fallback.clone(),
        };
        Self {
            locale: locale.to_string(),
            strings,
            fallback,
        }
    }

    pub fn from_file(locale: &str, path: &Path) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let parsed: HashMap<String, String> = serde_json::from_str(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let fallback: HashMap<String, String> =
            serde_json::from_str(EMBEDDED_EN).unwrap_or_default();
        Ok(Self {
            locale: locale.to_string(),
            strings: parsed,
            fallback,
        })
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.fallback.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_and_it_catalogs_have_identical_key_coverage() {
        let en = I18nCatalog::new("en");
        let it = I18nCatalog::new("it");
        for key in en.keys() {
            assert!(
                it.strings.contains_key(key),
                "Missing Italian translation for key: {key}"
            );
            assert!(!en.t(key).is_empty());
            assert!(!it.t(key).is_empty());
        }
    }
}
