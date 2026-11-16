//! Source-first editor foundations, independent of the GUI toolkit.
pub mod alignment_edit;
pub mod backend;
pub mod declaration_fields;
pub mod delete_edit;
pub mod diagnostic;
pub mod document;
pub mod element_edit;
pub mod file_state;
#[cfg(not(target_arch = "wasm32"))]
pub mod files;
pub mod graph_edit;
pub mod i18n;
pub mod inline_text;
pub mod membership_edit;
pub mod node_edit;
pub mod notation;
pub mod notation_edit;
pub mod placement_edit;
pub mod port_edit;
pub mod preview;
pub mod properties;
pub mod relation_edit;
pub mod relation_selection;
pub mod selection;
pub mod semantic_edit;
pub mod settings_edit;
pub mod source;
mod source_patch;
pub mod style_edit;
pub mod text_tools;

#[cfg(not(target_arch = "wasm32"))]
pub mod dependencies;
pub mod output;
#[cfg(not(target_arch = "wasm32"))]
pub mod recovery;
#[cfg(target_arch = "wasm32")]
#[path = "recovery_web.rs"]
pub mod recovery;
pub mod recovery_record;
pub mod source_link;
pub mod structure;
pub mod syntax_tools;

pub mod gesture_preview;

extern crate self as laml_ui;
pub mod app;
#[cfg(target_arch = "wasm32")]
pub(crate) mod browser_storage;
pub mod services;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;
