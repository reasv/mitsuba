FROM rust:1.51.0-slim-buster AS build
WORKDIR /usr/src
RUN apt update && apt install build-essential g++ pkg-config musl musl-tools -y
RUN rustup target add x86_64-unknown-linux-musl
RUN USER=root cargo new mitsuba
WORKDIR /usr/src/mitsuba
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY static ./static
COPY migrations ./migrations
COPY sqlx-data.json ./sqlx-data.json
COPY src ./src

ENV SQLX_OFFLINE="true"
RUN cargo install --target x86_64-unknown-linux-musl --path .

FROM scratch
COPY --from=build /usr/local/cargo/bin/mitsuba .
USER 1000
CMD ["./mitsuba"]