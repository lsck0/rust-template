FROM ghcr.io/catthehacker/ubuntu:act-24.04@sha256:19619a010b204f44c5340bff7d39b68551109fe6a56a53f75201ea91e05d4270

ARG RUSTUP_VERSION=1.29.2
ARG RUSTUP_SHA256=5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4

RUN curl -fsSL --retry 3 -o /tmp/rustup-init \
    "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" \
    && echo "${RUSTUP_SHA256}  /tmp/rustup-init" | sha256sum --check \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none \
    && rm /tmp/rustup-init

ENV PATH=/root/.cargo/bin:${PATH}
