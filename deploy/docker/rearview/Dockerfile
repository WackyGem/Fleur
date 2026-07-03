# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

WORKDIR /workspace

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY engines/Cargo.toml engines/Cargo.lock ./engines/
COPY engines/crates/furnace/Cargo.toml ./engines/crates/furnace/Cargo.toml
COPY engines/crates/furnace-core/Cargo.toml ./engines/crates/furnace-core/Cargo.toml
COPY engines/crates/furnace-io/Cargo.toml ./engines/crates/furnace-io/Cargo.toml
COPY engines/crates/rearview-core/Cargo.toml ./engines/crates/rearview-core/Cargo.toml
COPY engines/crates/rearview-server/Cargo.toml ./engines/crates/rearview-server/Cargo.toml
COPY engines/crates/rearview-portfolio-worker/Cargo.toml ./engines/crates/rearview-portfolio-worker/Cargo.toml
COPY engines ./engines

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cd engines && cargo fetch --locked

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cd engines \
    && cargo build --release --locked -p rearview-server -p rearview-portfolio-worker \
    && mkdir -p /out \
    && cp target/release/rearview-server /out/rearview-server \
    && cp target/release/rearview-portfolio-worker /out/rearview-portfolio-worker

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /out/rearview-server /usr/local/bin/rearview-server
COPY --from=builder /out/rearview-portfolio-worker /usr/local/bin/rearview-portfolio-worker
COPY .env.example ./.env.example
COPY engines/Cargo.toml ./engines/Cargo.toml
COPY pipeline/pyproject.toml ./pipeline/pyproject.toml
COPY engines/crates/rearview-core/config/metric_policy.yml ./engines/crates/rearview-core/config/metric_policy.yml
COPY pipeline/elt/models/marts ./pipeline/elt/models/marts

EXPOSE 34057

CMD ["rearview-server", "serve"]
