pub mod parsers;

use std::path::Path;

use tracing::debug;

pub fn path_to_collection_name(path: &Path) -> String {
    // If it's a git repository, use the repo name
    if path.join(".git").exists() {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            return name.to_string();
        }
    }

    debug!("Unable to generate collection_name");

    // Otherwise use the last component of the path
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "code-sherpa".to_string())
}
