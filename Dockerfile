# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.97.1
ARG CARGO_CHEF_VERSION=0.1.77

FROM rust:${RUST_VERSION}-bookworm AS chef
ARG CARGO_CHEF_VERSION
RUN cargo install cargo-chef --version "${CARGO_CHEF_VERSION}" --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --locked --release --bin rock-matching-server

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime
COPY --from=builder --chown=65532:65532 /app/target/release/rock-matching-server /bin/rock-matching-server
USER 65532:65532
EXPOSE 3000
ENTRYPOINT ["/bin/rock-matching-server"]
