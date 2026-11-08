# Pumpkin built from upstream: master eda69440e + PR #3962 head (0740d3f6), feature plugin-v0_2.
# The merge is redone here; its tree is identical to the one used for the results (see README).
# Build time is about 30 minutes on 1-2 cores.
FROM rust:1.98-trixie AS build
WORKDIR /src
COPY scripts/checkout-pumpkin.sh /checkout-pumpkin.sh
RUN /checkout-pumpkin.sh /src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release -p pumpkin --features plugin-v0_2 && \
    cp target/release/pumpkin /pumpkin-bin

FROM ubuntu:26.04
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates netcat-openbsd && \
    rm -rf /var/lib/apt/lists/*
COPY --from=build /pumpkin-bin /usr/local/bin/pumpkin
RUN groupadd -g 2613 pumpkin && \
    useradd -u 2613 -g pumpkin -m -d /pumpkin pumpkin && \
    chown -R pumpkin:pumpkin /pumpkin
WORKDIR /pumpkin
USER pumpkin:pumpkin
ENV RUST_BACKTRACE=1
EXPOSE 25565
ENTRYPOINT [ "pumpkin" ]
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD nc -z 127.0.0.1 25565 || exit 1
