# acres-report

How the `acres-cli` and `acres-tui` apps classify and report a failure.

A `Failure` is built from any error chain. It carries a stable `kind` (such as `cache.missing`), the message and its causes, a hint on what to do next and an exit code by class of failure. It is written as text, with `error:`, `caused by:` and `hint:` lines, or as one line of JSON.

```rust
use acres_report::Failure;

let error = std::io::Error::other("disk full");
let failure = Failure::from_error(&error);
eprint!("{}", failure.to_text());
std::process::exit(failure.exit_code.into());
```

The exit codes are in the `exit` module, and an app adds its own errors to the classification with `classify_with`. The [workspace README](https://github.com/dylan-stark/acres/blob/main/README.md) lists every exit code and covers the apps. Run `cargo doc --open -p acres-report` for the API docs.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT license](../LICENSE-MIT) at your option.
