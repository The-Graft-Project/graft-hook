FROM rust:1.83-slim-bookworm AS builder
# libgit2-dev and pkg-config are required for the 'git2' crate
RUN apt-get update && apt-get install -y pkg-config libssl-dev libgit2-dev cmake gcc
WORKDIR /app
COPY . .
RUN cargo build --release


FROM debian:bookworm-slim
# Runtime requirements
RUN apt-get update && apt-get install -y libssl3 libgit2-1.5 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/graft-hook . 
EXPOSE 3000
CMD ["./graft-hook"]