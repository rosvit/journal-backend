# Build stage
FROM rust:1.80.1-slim as builder
WORKDIR /app
ADD . /app
RUN cargo build --release

# Prod stage
FROM ubuntu:24.04
COPY --from=builder /app/target/release/journal-backend /
EXPOSE 8080
CMD ["/journal-backend"]
