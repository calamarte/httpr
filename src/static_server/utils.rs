use std::path::Path;

pub fn mime_by_ext(ext: &str) -> String {
    mime_guess::from_ext(ext).first_or_text_plain().to_string()
}

pub fn mime_by_path(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_text_plain()
        .to_string()
}
