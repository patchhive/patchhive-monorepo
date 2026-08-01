FROM rust:1.87-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY crates/patchhive-product-core ./crates/patchhive-product-core
COPY products/__PRODUCT_SLUG__/backend ./products/__PRODUCT_SLUG__/backend
WORKDIR /app/products/__PRODUCT_SLUG__/backend
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/products/__PRODUCT_SLUG__/backend/target/release/__PRODUCT_SLUG__ /usr/local/bin/__PRODUCT_SLUG__
RUN useradd --system --create-home --uid 10001 patchhive && chown patchhive:patchhive /usr/local/bin/__PRODUCT_SLUG__
WORKDIR /app
USER patchhive
CMD ["__PRODUCT_SLUG__"]
