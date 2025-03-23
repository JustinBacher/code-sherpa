use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, VariantNames};
use tree_sitter::Language;

#[derive(
    Debug, Clone, Serialize, Deserialize, EnumString, VariantNames, PartialEq, Eq, Hash, Display,
)]
pub enum SupportedParsers {
    #[serde(rename = "rs")]
    Rust,

    #[serde(rename = "go")]
    Go,

    #[serde(rename = "py")]
    Python,

    #[serde(rename = "js")]
    JavaScript,

    #[serde(rename = "ts")]
    TypeScript,

    #[allow(non_camel_case_types)]
    #[serde(rename = "tsx")]
    TSX,
}

impl SupportedParsers {
    pub fn language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::TSX => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}
