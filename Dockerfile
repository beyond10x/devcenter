FROM rust:1.97.0-bookworm@sha256:8fa55b2f3ddf97471ab6a767bfa3f37e6bad0986ba823e75fea57e2a2a5c3073 AS builder
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY web ./web
COPY openapi.json ./openapi.json
RUN cargo build --locked --release -p devcenter-app -p devcenterctl

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS server
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --home /nonexistent devcenter
COPY --from=builder /source/target/release/devcenter /usr/local/bin/devcenter
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/devcenter"]

FROM alpine/helm:3.19.0@sha256:aef9b56f64e866207d9591d0abd8f6d767b36aadd12edf68f8a719716d9d29c9 AS helm
FROM ghcr.io/oras-project/oras:v1.3.0@sha256:6ce045ce069a89934d6666b8b49f9c4c0145201bd6de6dbe2aee267814c55468 AS oras
FROM registry.k8s.io/kubectl:v1.34.1@sha256:59bafa07ff3a6d4b417e7633ddb9d79a9606ca98bf64bac080b3e65748669250 AS kubectl

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS ctl
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /source/target/release/devcenterctl /usr/local/bin/devcenterctl
COPY --from=helm /usr/bin/helm /usr/local/bin/helm
COPY --from=oras /bin/oras /usr/local/bin/oras
COPY --from=kubectl /bin/kubectl /usr/local/bin/kubectl
ENTRYPOINT ["/usr/local/bin/devcenterctl"]
