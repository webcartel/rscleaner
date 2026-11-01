#![forbid(unsafe_code)]

//! English and Russian catalogs for the CLI and TUI.

pub mod catalog;
pub mod language;
pub mod plural;

pub use catalog::Translator;
pub use language::Language;
pub use plural::{plural_en, plural_ru};
