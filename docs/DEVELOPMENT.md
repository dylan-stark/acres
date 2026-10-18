# Development

## Watching tests in a side pane

Run [bacon](https://dystroy.org/bacon/) in a tmux pane next to your editor so tests re-run on every
save, without switching windows.

From the pane your editor is in, open tmux's command prompt with `Ctrl-b :` and enter:

```
split-window -h -d -l 40% -c "#{pane_current_path}" "bacon test -- -p iiif; exec bash"
```

Or run it from any shell in the session (add `-t <session>:<window>` to target another window):

```sh
tmux split-window -h -d -l 40% -c "$PWD" "bacon test -- -p iiif; exec bash"
```

- `-h -l 40%` puts the pane on the right at 40% width; `-d` keeps focus in your editor.
- `-- -p iiif` is passed to `cargo test`. Change the crate, or drop it to test the whole workspace.
- `; exec bash` leaves a shell in the pane when bacon exits, rather than closing it.

### Useful keys

In the bacon pane:

| Key | Action |
|---|---|
| `f` | Show only failing tests |
| `s` | Toggle summary / full output |
| `b` | Toggle backtraces |
| `w` | Toggle line wrapping |
| `/` | Search the output |
| `t` / `c` | Switch to the test / clippy job |
| `q` | Quit bacon |

In tmux:

| Key | Action |
|---|---|
| `Ctrl-b` + arrow keys | Move between the editor and the bacon pane |
| `Ctrl-b z` | Zoom the current pane full-screen, and back |
| `Ctrl-b x` | Close the current pane |

## Diagnostics in Neovim with bacon-ls

[bacon-ls](https://github.com/crisidev/bacon-ls) is a language server that publishes `cargo clippy`
diagnostics into the editor, so `]d` jumps to the next error or warning. It reports compiler and
clippy diagnostics, not test failures; keep the bacon pane open for those.

With LazyVim's `lang.rust` extra, set this in `lua/config/options.lua`:

```lua
vim.g.lazyvim_rust_diagnostics = "bacon-ls"
```

The extra then enables the `bacon_ls` server for diagnostics and keeps rust-analyzer for
everything else. It only asks Mason to install `bacon`, so install `bacon-ls` too
(`:MasonInstall bacon-ls`, or add it to Mason's `ensure_installed`). Restart Neovim afterwards.
