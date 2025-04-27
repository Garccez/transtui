use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Locale {
    pub ui: HashMap<String, String>,
}

impl Locale {
    pub fn load(lang: &str) -> Result<Self> {
        let content = std::fs::read_to_string(format!("locales/{}/app.toml", lang))?;
        Ok(toml::from_str(&content)?)
    }

    // Correção do lifetime aqui
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.ui.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    pub fn get_with_params(&self, key: &str, params: &[(&str, &str)]) -> String {
        let mut text = self.get(key).to_string();
        for (k, v) in params {
            text = text.replace(&format!("{{{}}}", k), v);
        }
        text
    }
}
