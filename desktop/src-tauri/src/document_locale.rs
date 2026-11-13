//! System text only. Never apply translation to rendered HTML or user values.
use crate::db::Settings;
use serde_json::Value;
use std::sync::OnceLock;

pub fn language(settings: &Option<Settings>) -> &'static str {
    if settings.as_ref().and_then(|s| s.ui_language.as_deref()) == Some("en") { "en" } else { "ar" }
}

pub fn text<'a>(key: &'a str, settings: &Option<Settings>) -> &'a str {
    static MESSAGES: OnceLock<Value> = OnceLock::new();
    MESSAGES.get_or_init(|| serde_json::from_str(include_str!("../../src/domain/documentMessages.json"))
        .expect("valid shared document translation catalog"))
        .get(key).and_then(|v| v.get(language(settings))).and_then(Value::as_str).unwrap_or(key)
}
