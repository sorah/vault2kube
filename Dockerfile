FROM rust:1.43-buster as builder

RUN mkdir -p /build/src
RUN echo 'fn main() {}' > /build/src/main.rs
COPY Cargo.toml Cargo.lock  /build/
WORKDIR /build
RUN cargo build --release --locked

COPY src /build/src
RUN cargo build --release --locked

FROM debian:buster-slim

ENV LANG C.UTF-8
ENV DEBIAN_FRONTEND noninteractive

RUN apt-get update && apt-get upgrade -y --no-install-recommends --no-install-suggests \
  && apt-get install --no-install-recommends --no-install-suggests -y \
    libssl1.1 ca-certificates \
  && apt-get remove --purge --auto-remove -y && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/vault2kube /usr/bin/

ENTRYPOINT /usr/bin/vault2kube
CMD ["run"]
