FROM rust AS builder
WORKDIR /app
RUN rustup target add $(uname -m)-unknown-linux-musl
RUN apt-get update
RUN apt-get install -y musl-tools


RUN USER=root cargo new kube-engine-directory
WORKDIR /app/kube-engine-directory
COPY ./dummy.rs ./src/main.rs
COPY Cargo.toml ./
COPY Cargo.lock ./
RUN cargo build --release --target $(uname -m)-unknown-linux-musl

COPY src ./src
RUN cargo build  --release --target $(uname -m)-unknown-linux-musl
RUN mv ./target/$(uname -m)-unknown-linux-musl/release/kube-engine-directory /

FROM scratch as runtime
WORKDIR /
COPY --from=builder /kube-engine-directory /
ENTRYPOINT ["/kube-engine-directory"]