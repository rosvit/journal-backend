# Build stage
FROM rust:1.80.1-slim as builder
WORKDIR /app
ADD . /app
RUN sed -i '2d' .env
RUN cargo build --release

# Prod stage
FROM ubuntu:24.04
COPY --from=builder /app/target/release/journal-backend /
COPY --from=builder /app/.env /
EXPOSE 8080
CMD ["/journal-backend"]
