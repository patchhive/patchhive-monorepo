FROM rust:1.87-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev git && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y git ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/repo-reaper /usr/local/bin/repo-reaper
RUN useradd --system --create-home --uid 10001 patchhive && chown patchhive:patchhive /usr/local/bin/repo-reaper
WORKDIR /app
USER patchhive
CMD ["repo-reaper"]
