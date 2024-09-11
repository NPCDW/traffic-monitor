FROM rust:latest AS rust-build

RUN apt-get update

WORKDIR /usr/src/traffic-monitor
COPY ./ ./
RUN cargo build --release



FROM node:20.11.1 AS node-build

WORKDIR /usr/src/
RUN git clone https://github.com/NPCDW/traffic-monitor-web.git
WORKDIR /usr/src/traffic-monitor-web
RUN npm install -g pnpm
RUN pnpm install
RUN pnpm run build



FROM debian:bookworm-slim

WORKDIR /traffic-monitor

COPY --from=node-build /usr/src/traffic-monitor-web/dist /ui
COPY --from=rust-build /usr/src/traffic-monitor/target/release/traffic-monitor /usr/local/bin/traffic-monitor

RUN apt-get update
RUN apt-get install -y openssl ca-certificates

ENTRYPOINT ["/usr/local/bin/traffic-monitor"]