---
description: Run make check and summarize any failures
allowed-tools: Bash(make:*)
---

Run `make check`. If it passes, say so in one line.

If it fails, report which step failed (toolchain, reformat-check, lint, test, or docs), the relevant `file:line` locations and messages, and a suggested fix for each. Do not apply fixes unless asked.
