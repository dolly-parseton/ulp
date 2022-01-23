# Create the build container to compile ULP on
FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

WORKDIR /usr/src/ulp
COPY ./ .
RUN cargo build --target x86_64-unknown-linux-musl --release

# Create the execution container by copying the compiled hello world to it and running it
FROM ubuntu
RUN apt-get update && rm -rf /var/lib/apt/lists/*

WORKDIR /bin
COPY --from=builder /usr/src/ulp/target/x86_64-unknown-linux-musl/release/ulp ./

# Create directories
RUN mkdir -p /data
RUN mkdir -p /output

ENV RUST_LOG="ulp"
EXPOSE 3030
CMD ["/bin/ulp"]
