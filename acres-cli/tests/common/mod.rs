//! Shared helpers for the CLI tests.

use std::{
    process::Command,
    sync::atomic::{AtomicU32, Ordering},
};

use assert_cmd::prelude::*;

/// The `acres-cli` binary, with a rate limit of its own, so the tests neither wait on each other
/// nor spend the tokens of whoever runs them.
pub fn acres_cli() -> Command {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let mut cmd = Command::cargo_bin("acres-cli").expect("the binary is built");
    cmd.env(
        "ACRES_STATE_DIR",
        format!(
            "{}/state-{}-{}",
            env!("CARGO_TARGET_TMPDIR"),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ),
    );
    cmd
}
