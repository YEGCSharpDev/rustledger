# Stage 1: Builder

FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/rustledger

# Copy the source code
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /data

COPY --from=builder /usr/src/rustledger/target/release/rledger-* /usr/local/bin/

# (Optional) If you want the 'bean-*' aliases (compatibility mode), create symlinks
 RUN ln -s /usr/local/bin/rledger-check /usr/local/bin/bean-check && \
     ln -s /usr/local/bin/rledger-query /usr/local/bin/bean-query && \
     ln -s /usr/local/bin/rledger-report /usr/local/bin/bean-report

#removing entry point, using container like a toolbox
CMD ["rledger-query", "--help"]
