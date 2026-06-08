# Build the whole workspace once in `builder`, then select a binary per stage.
# docker-compose picks the stage via `build.target: <auth|user|gateway>`.
FROM rust:1-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release --bins

FROM debian:bookworm-slim AS runtime-base
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM runtime-base AS auth
COPY --from=builder /app/target/release/auth-service /usr/local/bin/app
ENTRYPOINT ["app"]

FROM runtime-base AS user
COPY --from=builder /app/target/release/user-service /usr/local/bin/app
ENTRYPOINT ["app"]

FROM runtime-base AS gateway
COPY --from=builder /app/target/release/gateway /usr/local/bin/app
ENTRYPOINT ["app"]
