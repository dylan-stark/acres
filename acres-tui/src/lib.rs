//! acres-tui is a terminal UI for exploring artworks from the Art Institute of Chicago's
//! [public APIs](https://api.artic.edu/docs/).
//!
//! The binary is a thin `main` over this library, which the frame benches also build on.

// Following is meant to handle clippy bug about uneeded return
#![allow(clippy::needless_return)]

pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod config;
pub mod entry;
pub mod errors;
pub mod explorer;
pub mod logging;
pub mod saved;
pub mod tui;
