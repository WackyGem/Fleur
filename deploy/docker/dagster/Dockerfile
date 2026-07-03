# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS furnace-builder

WORKDIR /workspace

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
    && cargo build --release --locked -p furnace \
    && mkdir -p /out \
    && cp target/release/furnace /out/furnace

FROM python:3.12.13-slim-bookworm

ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV PATH="/app/pipeline/.venv/bin:${PATH}"

WORKDIR /app/pipeline

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && pip install --no-cache-dir uv

COPY pipeline/pyproject.toml pipeline/uv.lock ./
COPY pipeline/contract_tools/pyproject.toml ./contract_tools/pyproject.toml
COPY pipeline/scheduler/pyproject.toml ./scheduler/pyproject.toml

RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --all-packages --no-install-workspace

COPY pipeline/contract_tools ./contract_tools
COPY pipeline/scheduler ./scheduler
COPY pipeline/elt ./elt
COPY pipeline/contracts ./contracts
COPY pipeline/migrate ./migrate
COPY --from=furnace-builder /out/furnace /usr/local/bin/furnace

RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --all-packages

CMD ["uv", "run", "dagster-webserver", "-h", "0.0.0.0", "-p", "3000", "--path-prefix", "/dagster", "-m", "scheduler.definitions", "-d", "/app/pipeline/scheduler"]
