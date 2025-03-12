FROM rust AS builder

# Set the working directory
WORKDIR /app

# Copy the source files into the builder
COPY src/ src/
COPY Cargo.lock Cargo.lock 
COPY Cargo.toml Cargo.toml

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/eshop-orders /usr/local/bin/eshop-orders

WORKDIR /app

EXPOSE 3000

CMD ["/usr/local/bin/eshop-orders"]