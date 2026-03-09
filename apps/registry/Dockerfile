ARG RUST_VERSION=1.92
ARG ALPINE_VERSION=3.23

FROM rust:${RUST_VERSION}-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
# Registry is excluded from the root workspace — standalone build, no stubs needed
COPY . .
RUN cargo build --release

FROM alpine:${ALPINE_VERSION}
RUN addgroup -S app && adduser -S app -G app && \
    mkdir -p /app/data/registry && chown -R app:app /app
COPY --from=builder /app/target/release/allsource-registry /usr/local/bin/
USER app
WORKDIR /app
EXPOSE 3901
HEALTHCHECK --interval=30s --timeout=5s CMD wget -qO- http://localhost:3901/health || exit 1
CMD ["allsource-registry"]
