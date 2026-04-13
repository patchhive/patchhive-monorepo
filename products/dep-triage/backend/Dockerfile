FROM rust:1.78-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/dep-triage /usr/local/bin/dep-triage
RUN useradd --system --create-home --uid 10001 patchhive && chown patchhive:patchhive /usr/local/bin/dep-triage
WORKDIR /app
USER patchhive
CMD ["dep-triage"]
