use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

/// A Sunday on La Grande Jatte (843x565), shared with the TUI's fixtures.
const IMAGE: &str =
    "../acres-tui/tests/fixtures/iiif/2d484387-2509-5e8e-2c43-22f9981972eb/full/843,/0/default.jpg";

fn ascii_art() -> Result<Command, Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.args(["ascii-art", IMAGE, "--width", "40"])
        .env_remove("NO_COLOR")
        .env("COLORTERM", "truecolor");
    Ok(cmd)
}

#[test]
fn color_never_writes_plain_text() -> Result<(), Box<dyn std::error::Error>> {
    let output = ascii_art()?.args(["--color", "never"]).output()?;
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        13,
        "40 columns of an 843x565 image at 2:1 cells"
    );
    assert!(lines.iter().all(|line| line.len() == 40));
    assert!(!text.contains('\x1b'));
    Ok(())
}

#[test]
fn color_256_writes_indexed_escapes() -> Result<(), Box<dyn std::error::Error>> {
    ascii_art()?
        .args(["--color", "256"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;5;"))
        .stdout(predicate::str::contains("\x1b[38;2;").not());
    Ok(())
}

#[test]
fn color_truecolor_is_forced_through_a_pipe() -> Result<(), Box<dyn std::error::Error>> {
    ascii_art()?
        .args(["--color", "truecolor"])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;2;"));
    Ok(())
}

#[test]
fn color_auto_is_off_through_a_pipe() -> Result<(), Box<dyn std::error::Error>> {
    // The test harness captures stdout, so it is never a terminal here.
    ascii_art()?
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b").not());
    Ok(())
}

#[test]
fn ramp_takes_custom_characters() -> Result<(), Box<dyn std::error::Error>> {
    let output = ascii_art()?
        .args(["--color", "never", "--ramp", " #"])
        .output()?;
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)?;
    assert!(text.chars().all(|c| matches!(c, ' ' | '#' | '\n')));
    Ok(())
}

#[test]
fn ramp_rejects_unprintable_characters() -> Result<(), Box<dyn std::error::Error>> {
    ascii_art()?
        .args(["--ramp", "\u{e9}"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not printable ASCII"));
    Ok(())
}

#[test]
fn timings_go_to_stderr_after_the_art() -> Result<(), Box<dyn std::error::Error>> {
    let plain = ascii_art()?.args(["--color", "never"]).output()?;
    let timed = ascii_art()?
        .args(["--color", "never", "--timings"])
        .output()?;
    assert!(timed.status.success());
    assert_eq!(timed.stdout, plain.stdout);
    let stderr = String::from_utf8(timed.stderr)?;
    for stage in ["read", "decode", "load", "render", "write", "total"] {
        assert!(
            stderr.lines().any(|line| line.starts_with(stage)),
            "{stage} missing from {stderr:?}"
        );
    }
    Ok(())
}
