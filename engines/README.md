# mono-fleur Rust Engines

This directory contains the Rust workspace for backend compute engines.

The initial workspace is intentionally only a project skeleton. Business logic,
ClickHouse integration, Dagster integration, and indicator implementations will
be added in later changes.

## Workspace

```text
engines/
├── Cargo.toml
└── crates/
    ├── furnace/
    ├── furnace-core/
    └── furnace-io/
```

## Checks

Run from `engines/`:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
