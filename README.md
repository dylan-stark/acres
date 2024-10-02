# aic-tui

[![CI](https://github.com/dylan-stark/aic-tui/workflows/CI/badge.svg)](https://github.com/dylan-stark/aic-tui/actions)

A TUI for browsing Art Institute Chicago's public domain artworks as ASCII art.

<img src="https://vhs.charm.sh/vhs-qa7lf8ndcehbzprwDjPg.gif" alt="Made with VHS" width="600">
  <a href="https://vhs.charm.sh">
    <!-- <img src="https://stuff.charm.sh/vhs/badge.svg"> -->
  </a>
</img>

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
