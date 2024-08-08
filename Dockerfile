FROM rust:latest AS rust-build

RUN apt-get update

WORKDIR /usr/src/traffic-monitor
COPY ./ ./
RUN cargo build --release




FROM debian:bookworm-slim

WORKDIR /traffic-monitor

COPY --from=rust-build /usr/src/traffic-monitor/target/release/traffic-monitor /usr/local/bin/traffic-monitor

RUN apt-get update
RUN apt-get install -y openssl ca-certificates

ENTRYPOINT ["/usr/local/bin/traffic-monitor"]