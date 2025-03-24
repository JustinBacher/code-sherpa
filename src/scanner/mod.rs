mod results;
#[allow(clippy::module_inception)]
mod scanner;

#[allow(unused_imports)]
pub use results::ScanResults;
pub use scanner::{CodebaseScanner, ScannerConfig};
