# Ambiente de build do AppImage. Ubuntu 22.04 (glibc 2.35) para o binário
# rodar na maioria das distros atuais.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    # linuxdeploy é um AppImage; dentro do container não há FUSE
    APPIMAGE_EXTRACT_AND_RUN=1

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential curl wget file ca-certificates pkg-config git \
        libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
        libayatana-appindicator3-dev librsvg2-dev libssl-dev \
        patchelf squashfs-tools xdg-utils \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal \
    && cargo install tauri-cli --version "^2" --locked

WORKDIR /app
CMD ["cargo", "tauri", "build", "--bundles", "appimage"]
