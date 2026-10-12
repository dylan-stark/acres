#!/usr/bin/env bash
# PostToolUse hook: run `make reformat` after an edit to a Rust file.
file=$(jq -r '.tool_input.file_path // empty')
case "$file" in
  *.rs) cd "$CLAUDE_PROJECT_DIR" && make reformat ;;
esac
exit 0
