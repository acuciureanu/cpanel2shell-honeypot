# Multi-stage build for cpanel2shell-honeypot
FROM rust:1.87-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cpanel2shell-honeypot /usr/local/bin/
COPY data/ /app/data/
WORKDIR /app
EXPOSE 2083 2087
ENTRYPOINT ["cpanel2shell-honeypot"]
