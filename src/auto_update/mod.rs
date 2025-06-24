// src/auto_update/mod.rs

pub mod updater;
pub mod llm_parsers;
pub mod text_utils;
pub mod code_gen;
pub mod doc_gen;
pub mod cargo_toml_updater; // Added new module

// Optional: re-export key items if needed directly from `crate::auto_update::Item`
pub use updater::{AutoUpdater, UpdateContext};

#[cfg(test)]
mod tests;
