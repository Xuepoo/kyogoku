use fluent_bundle::bundle::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use log::{error, warn};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use unic_langid::LanguageIdentifier;

#[derive(RustEmbed)]
#[folder = "locales"]
struct Asset;

pub struct I18n {
    bundles: HashMap<String, FluentBundle<FluentResource, IntlLangMemoizer>>,
    current_lang: String,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        let mut bundles = HashMap::new();
        let langs = vec!["en-US", "zh-CN", "ja-JP"];

        for lang in langs {
            let lang_id: LanguageIdentifier = lang.parse().expect("Failed to parse language ID");
            let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);

            let file_path = format!("{}/main.ftl", lang);
            if let Some(file) = Asset::get(&file_path) {
                if let Ok(source) = std::str::from_utf8(file.data.as_ref()) {
                    match FluentResource::try_new(source.to_string()) {
                        Ok(res) => {
                            if let Err(errors) = bundle.add_resource(res) {
                                warn!("Failed to add FTL resource for {}: {:?}", lang, errors);
                            }
                        }
                        Err((_res, errors)) => {
                            error!("Failed to parse FTL for {}: {:?}", lang, errors);
                        }
                    }
                }
            }
            bundles.insert(lang.to_string(), bundle);
        }

        Self {
            bundles,
            current_lang: "en-US".to_string(),
        }
    }

    pub fn set_lang(&mut self, lang: &str) {
        if self.bundles.contains_key(lang) {
            self.current_lang = lang.to_string();
        } else {
            warn!(
                "Language {} not supported, keeping {}",
                lang, self.current_lang
            );
        }
    }

    pub fn t(&self, key: &str, args: Option<&FluentArgs>) -> String {
        if let Some(bundle) = self.bundles.get(&self.current_lang) {
            if let Some(msg) = bundle.get_message(key) {
                if let Some(pattern) = msg.value() {
                    let mut errors = vec![];
                    let value = bundle.format_pattern(pattern, args, &mut errors);
                    return value.to_string();
                }
            }
        }
        key.to_string()
    }

    pub fn get_lang(&self) -> &str {
        &self.current_lang
    }
}

static INSTANCE: OnceLock<Mutex<I18n>> = OnceLock::new();

pub fn init() {
    INSTANCE.get_or_init(|| Mutex::new(I18n::new()));
}

pub fn set_locale(lang: &str) {
    if let Some(lock) = INSTANCE.get() {
        if let Ok(mut i18n) = lock.lock() {
            i18n.set_lang(lang);
        }
    }
}

pub fn get_locale() -> String {
    if INSTANCE.get().is_none() {
        init();
    }
    if let Some(lock) = INSTANCE.get() {
        if let Ok(i18n) = lock.lock() {
            return i18n.get_lang().to_string();
        }
    }
    "en-US".to_string()
}

pub fn translate(key: &str) -> String {
    if INSTANCE.get().is_none() {
        init();
    }
    if let Some(lock) = INSTANCE.get() {
        if let Ok(i18n) = lock.lock() {
            return i18n.t(key, None);
        }
    }
    key.to_string()
}
