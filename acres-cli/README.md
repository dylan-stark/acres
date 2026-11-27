# acres-cli

A command line for the [Art Institute of Chicago's public API](https://api.artic.edu/docs/): fetch and search artworks, build IIIF image URLs, and draw images as ASCII or block art in the terminal.

```sh
acres-cli artworks-search --q monet > monet.json
acres-cli artwork 27992 | acres-cli iiif - --to bytes | acres-cli ascii-art - --mode half
```

JSON goes to stdout and everything else to stderr. Requests share one rate limit and an on-disk cache, `--offline` answers from the cache alone, and failures have stable kinds, hints and exit codes, with `--error-format json` for scripts and agents.

The [workspace README](https://github.com/dylan-stark/acres/blob/main/README.md#the-cli) covers each command, the configuration and troubleshooting.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT license](../LICENSE-MIT) at your option.
