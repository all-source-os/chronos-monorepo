ARG RUST_VERSION=1.92
ARG ALPINE_VERSION=3.23

FROM rust:${RUST_VERSION}-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/registry/ apps/registry/
# Stubs for other workspace members (must satisfy Cargo.toml declarations)
COPY apps/core/Cargo.toml apps/core/
COPY tooling/performance/Cargo.toml tooling/performance/
COPY sdks/rust/Cargo.toml sdks/rust/
RUN mkdir -p apps/core/src apps/core/benches apps/core/tests/stress_tests \
            tooling/performance/src sdks/rust/src && \
    echo 'fn main() {}' > apps/core/src/main.rs && \
    echo '' > apps/core/src/lib.rs && \
    echo 'fn main() {}' > apps/core/benches/performance_benchmarks.rs && \
    echo 'fn main() {}' > apps/core/tests/stress_tests/seven_day_stress.rs && \
    echo 'fn main() {}' > tooling/performance/src/main.rs && \
    echo '' > sdks/rust/src/lib.rs
RUN cargo build --release -p allsource-registry

FROM alpine:${ALPINE_VERSION}
RUN addgroup -S app && adduser -S app -G app && \
    mkdir -p /app/data/registry && chown -R app:app /app
COPY --from=builder /app/target/release/allsource-registry /usr/local/bin/
USER app
WORKDIR /app
EXPOSE 3901
HEALTHCHECK --interval=30s --timeout=5s CMD wget -qO- http://localhost:3901/health || exit 1
CMD ["allsource-registry"]
