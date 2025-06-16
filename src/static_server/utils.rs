#![allow(dead_code)] // TODO: check alternative is too slow

use once_cell::sync::Lazy;
use reqwest::header;
use serde_json::Value;
use std::{collections::HashMap, path::Path};
use tokio::sync::{Mutex, RwLock};

const EXT_URL: &str = "https://file-extension.p.rapidapi.com/details";

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let mut headers = header::HeaderMap::with_capacity(2);

    let mut api_key = header::HeaderValue::from_static(env!("EMBEDDED_API_KEY"));
    api_key.set_sensitive(true);

    headers.insert(
        "x-rapidapi-host",
        header::HeaderValue::from_static("file-extension.p.rapidapi.com"),
    );
    headers.insert("x-rapidapi-key", api_key);

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("Not fail!")
});

static FILE_TYPE_CACHE: Lazy<RwLock<HashMap<String, Value>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static API_CALL_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub fn mime_by_ext(ext: &str) -> String {
    mime_guess::from_ext(ext).first_or_text_plain().to_string()
}

pub fn mime_by_path(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_text_plain()
        .to_string()
}

pub async fn type_by_ext(ext: &str) -> Value {
    if let Some(value) = FILE_TYPE_CACHE.read().await.get(ext) {
        return value.clone();
    }

    let _ = API_CALL_LOCK.lock().await;

    if let Some(value) = FILE_TYPE_CACHE.read().await.get(ext) {
        return value.clone();
    }

    let data: Value = CLIENT
        .get(EXT_URL)
        .query(&[("extension", ext)])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    FILE_TYPE_CACHE
        .write()
        .await
        .insert(ext.to_string(), data.clone());

    data
}
