# Stage 1: Build
FROM rust:1.79.0-slim-buster AS build

WORKDIR /usr/src

# Install necessary dependencies
RUN apt update && apt install -y \
    build-essential \
    g++ \
    pkg-config \
    musl \
    musl-tools

# Add the target for musl
RUN rustup target add x86_64-unknown-linux-musl

# Create a new Rust project (for caching dependencies)
RUN USER=root cargo new mitsuba
WORKDIR /usr/src/mitsuba

# Copy Cargo files and build dependencies (cached if Cargo files don't change)
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Copy the rest of the source files
COPY static ./static
COPY migrations ./migrations
COPY sqlx-data.json ./sqlx-data.json
COPY src ./src

# Set environment variable for offline mode
ENV SQLX_OFFLINE="true"

# Build the final release
RUN cargo build --release
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Stage 2: Runtime
FROM alpine:3.20.1

# Copy the compiled binary from the build stage
COPY --from=build /usr/local/cargo/bin/mitsuba .

# Create a directory for data and set it as a volume
RUN mkdir /data
VOLUME /data

# Set the entrypoint for the container
CMD ["./mitsuba", "start"]
