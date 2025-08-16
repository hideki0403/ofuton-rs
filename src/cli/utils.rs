use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use regex::Regex;

/// content-dispositionのfilenameで正規化が必要な文字列
/// See: https://datatracker.ietf.org/doc/html/rfc8187#section-3.2.1 attr-char
pub static FILENAME_NORMALIZE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^A-Za-z0-9!#$&+-.^_`|~]").expect("Failed to compile regex")
});

pub fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({per_sec}, ETA: {eta})")
        .unwrap()
        .progress_chars("#>-"));

    return pb;
}
