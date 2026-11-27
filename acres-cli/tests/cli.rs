use assert_cmd::prelude::*;
use predicates::prelude::predicate;
use std::process::Command;

#[test]
fn requires_subcommand() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;

    cmd.assert().failure().stderr(predicate::str::contains(
        "requires a subcommand but one was not provided",
    ));
    Ok(())
}

#[test]
fn unsupported_subcommand() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.arg("artists");

    cmd.assert().failure().stderr(predicate::str::contains(
        "unrecognized subcommand 'artists'",
    ));
    Ok(())
}

#[test]
fn an_offline_miss_exits_4_with_a_hint() -> Result<(), Box<dyn std::error::Error>> {
    let cache = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_CACHE_DIR", cache.path())
        .env("ACRES_STATE_DIR", cache.path())
        .env_remove("RUST_BACKTRACE")
        .args(["--offline", "artwork", "27992"]);

    cmd.assert()
        .code(4)
        .stdout("")
        .stderr(predicate::str::starts_with(
            "error: not cached while offline",
        ))
        .stderr(predicate::str::contains("hint: go online"));
    Ok(())
}

#[test]
fn json_errors_are_one_line_with_a_kind() -> Result<(), Box<dyn std::error::Error>> {
    let cache = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_CACHE_DIR", cache.path())
        .env("ACRES_STATE_DIR", cache.path())
        .args(["--offline", "--error-format", "json", "artwork", "27992"]);

    let output = cmd.output()?;
    assert_eq!(output.status.code(), Some(4));
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    let json: serde_json::Value = serde_json::from_str(&stderr)?;
    assert_eq!(json["kind"], "cache.missing");
    assert_eq!(json["exit_code"], 4);
    assert!(json["hint"].is_string());
    Ok(())
}

#[test]
fn usage_errors_follow_the_error_format() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_ERROR_FORMAT", "json").arg("artists");

    let output = cmd.output()?;
    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(json["kind"], "usage");
    Ok(())
}

#[test]
fn a_bad_environment_value_exits_7() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("acres-cli")?;
    cmd.env("ACRES_OFFLINE", "maybe")
        .env_remove("RUST_BACKTRACE")
        .args(["artwork", "1"]);

    cmd.assert()
        .code(7)
        .stderr(predicate::str::contains("invalid configuration"))
        .stderr(predicate::str::contains("offline"));
    Ok(())
}

#[test]
fn verbose_logs_each_request_and_quiet_logs_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let cache = assert_fs::TempDir::new()?;
    let run = |flag: &str| -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::cargo_bin("acres-cli")?
            .env("ACRES_CACHE_DIR", cache.path())
            .env("ACRES_STATE_DIR", cache.path())
            .env_remove("RUST_LOG")
            .env_remove("RUST_BACKTRACE")
            .args([flag, "--offline", "artwork", "27992"])
            .output()?;
        Ok(String::from_utf8(output.stderr)?)
    };

    let verbose = run("-v")?;
    assert!(verbose.contains("request failed"), "{verbose}");
    assert!(verbose.contains(r#"kind="cache.missing""#), "{verbose}");

    let quiet = run("-q")?;
    assert!(!quiet.contains("request failed"), "{quiet}");
    assert!(
        quiet.starts_with("error: not cached while offline"),
        "{quiet}"
    );
    Ok(())
}
