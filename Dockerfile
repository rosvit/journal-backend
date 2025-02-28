# Build stage
FROM rust:1.82.0-slim as builder
WORKDIR /app
ADD . /app
RUN sed -i '/^DATABASE_URL/d' .env
RUN cargo build --release

# Prod stage
FROM ubuntu:24.04
COPY --from=builder /app/target/release/journal-backend /
COPY --from=builder /app/.env /
COPY --from=builder /app/migrations /migrations
EXPOSE 8080
CMD ["/journal-backend"]
