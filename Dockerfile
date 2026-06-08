# Multi-stage Nobel-level build for QuantumCart Backend
FROM rust:1.78-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin quantum_cart_server

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/quantum_cart_server /usr/local/bin/
COPY migrations /app/migrations

ENV RUST_LOG=info
EXPOSE 3000
CMD ["quantum_cart_server"]