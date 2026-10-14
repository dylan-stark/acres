# 0003. Split IIIF into its own crate

- Status: Accepted
- Date: retroactive (recorded 2026-10-14)

## Context

The AIC serves images through IIIF Image API 2.0. Building those URLs (region, size, rotation, quality, format) is generic and unrelated to the AIC's artwork API, but originally lived in the monolithic `aic-tui` crate.

## Decision

Put IIIF URL construction in a standalone `iiif` crate with no workspace dependencies. `acres` depends on it for artwork image URLs; `acres-cli` and `acres-tui` use it directly.

## Consequences

- Dependencies point strictly downward: `iiif` -> `acres` -> `acres-cli`/`acres-tui`. Never add an upward dependency.
- `iiif` can be documented, tested and published independently.
- Cross-crate changes to image requests require touching more than one crate.
