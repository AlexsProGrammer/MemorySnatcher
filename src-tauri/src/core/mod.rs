pub mod downloader;
pub mod geocoder;
pub mod media;
pub mod parser;
pub mod processor;
pub mod state;
pub mod zip_hunter;

use std::path::Path;

pub fn sqlite_url_from_path(path: &Path) -> String {
	let normalized = path.to_string_lossy().replace('\\', "/");
	format!("sqlite:///{}", normalized.trim_start_matches('/'))
}
