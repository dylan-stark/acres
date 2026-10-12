#!/usr/bin/env bash
# PostToolUse hook: run `cargo fmt` after an edit to a Rust file.
file=$(jq -r '.tool_input.file_path // empty')
case "$file" in
  *.rs) cd "$CLAUDE_PROJECT_DIR" && cargo fmt --all ;;
esac
exit 0
