pub use crate::error::Error::{self, *};

pub type Result<T> = std::result::Result<T, Error>;

#[allow(unused_imports)]
pub use std::format as f;
