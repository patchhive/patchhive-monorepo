FROM rust:1.82-bookworm AS builder

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/patchhive-backend /usr/local/bin/patchhive-backend

ENV PATCHHIVE_BIND_ADDR=0.0.0.0:8100
ENV PATCHHIVE_PRODUCTS=all

EXPOSE 8100

CMD ["patchhive-backend"]

