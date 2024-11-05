# aic-tui

[![CI](https://github.com/dylan-stark/aic-tui/workflows/CI/badge.svg)](https://github.com/dylan-stark/aic-tui/actions)

A TUI for browsing Art Institute Chicago's public domain artworks as ASCII art.

![Made with VHS](https://vhs.charm.sh/vhs-3r9vucoQZBithIQT3jG0n2.gif)

Built in Rust with the amazing [ratatui.rs](https://ratatui.rs) crate.

## Usage

This app lets you search and browse artwork via the [Art Institute of Chicago's public API](https://www.artic.edu/open-access/public-api) as ASCII art.

Start up the app by providing a search query, say for "waves":

```
aic-tui -q waves
```

This will search the [`/artworks` collection](https://api.artic.edu/docs/#collections) and when it starts up you should see a box showing the names of the artworks that matched your query.

At this point, you're in *browse* mode.
You can scroll the list using the up and down arrow keys (or `k` and `j` keys).
When you find something you want to look at, press enter (or `x`), to render the piece in the background.
And you can press escape to close this selection box.
(The full set of keybindings is [here](#browse-mode).)

When you close the selection box you are in *view* model.
You can sit back and enjoy the art or press escape to toggle back to browse mode to pick something else.
And if you're done you can press `q` (or `ctrl+d` or `ctrl+c`) to quit the application.
The full set of keybindings is [here](#view-mode).

Check out the help for all of the options.

```
$ aic-tui -h             
A TUI for browsing Art Institute Chicago's public domain artworks as ASCII art

Usage: aic-tui [OPTIONS]

Options:
  -q, --q <STRING>          Your search query [default: waves]
  -t, --tick-rate <FLOAT>   Tick rate, i.e. number of ticks per second [default: 4]
  -f, --frame-rate <FLOAT>  Frame rate, i.e. number of frames per second [default: 60]
  -h, --help                Print help
  -V, --version             Print version
```

## Configuration and data

We use `.config/` and `.data/` for configuration and data files.
You can find the directories at the bottom of the print-out from `aic-tui --version`.
And you can set them by setting the `AIC_TUI_CONFIG` or `AIC_TUI_DATA` environment variables, resp.

A quick way to set these to the local `.data/` and `.config/` folders is to use [direnv](https://direnv.net); e.g., with the following for `zsh`:

```
$ eval "$(direnv hook zsh)" 
```

### Keybindings

#### Browse mode

Default keybindings in *browse* mode:

| Key | Action |
|-----|--------|
| Down arrow or `j` | Move down to the next item in the list |
| Up arrow or `k` | Move up to the previous item in the list |
| Enter or `x` | Select this item |
| Escape | Switch to *view* mode to see entire artwork |

#### View mode

Default keybindings in *view* model:

| Key | Action |
|-----|--------|
| Escape | Switch to *browse* mode to select another piece |
| `q` or `CTRL+C` or `CTRL+D` | Quit the application |
| `CTRL+Z` | Suspend the application |

## Logs

If you're interested in the logs, check out `aic-tui.log` under your [`./data`](#configuration-and-data) directory.
The default log level will be `INFO`, but you can set it to whatever you want with the `AIC_TUI_LOGLEVEL` environment variable.
