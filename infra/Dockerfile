# syntax=docker/dockerfile:1.5

# Build on Debian 12 (bookworm) so the resulting binary is compatible with the
# distroless Debian 12 runtime glibc.
ARG RUST_IMAGE=rust:1.91.1-bookworm
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian12:nonroot@sha256:8f02d47496256aca25168c508024a60d81af996bbd2d89db4cf5cdf13dff2821

FROM ${RUST_IMAGE} AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

ARG BIN_NAME=transfer-legacy-api
RUN cargo build --release -p ${BIN_NAME}

FROM ${RUNTIME_IMAGE} AS runtime
WORKDIR /app

ARG BIN_NAME=transfer-legacy-api
COPY --from=builder /app/target/release/${BIN_NAME} /app/${BIN_NAME}
COPY --chown=nonroot:nonroot docs/templates /app/docs/templates

USER nonroot:nonroot
EXPOSE 8080
CMD ["/app/transfer-legacy-api"]
