# aic-tui

[![CI](https://github.com/dylan-stark/aic-tui/workflows/CI/badge.svg)](https://github.com/dylan-stark/aic-tui/actions)

A TUI for browsing Art Institute Chicago's public domain artworks as ASCII art.

![Made with VHS](https://vhs.charm.sh/vhs-3r9vucoQZBithIQT3jG0n2.gif)

Built in Rust with the amazing [ratatui.rs](https://ratatui.rs) crate.

## Run

```
aic-tui -q waves
```

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

## Logs

If you're interested in the logs, check out `aic-tui.log` under your [`./data`](#configuration-and-data) directory.
The default log level will be `INFO`, but you can set it to whatever you want with the `AIC_TUI_LOGLEVEL` environment variable.
