#!/usr/bin/env bash
# Gera o AppImage dentro de um container Ubuntu 22.04.
# Resultado: dist/*.AppImage
set -euo pipefail
cd "$(dirname "$0")/.."

docker build --network=host -t raycopy-appimage -f packaging/Dockerfile packaging

mkdir -p dist
# Target e cache do cargo ficam em volumes nomeados: não misturam com o
# target/ local (outra glibc) e deixam os builds seguintes rápidos.
docker run --rm --network=host \
    -v "$PWD":/app \
    -v raycopy-cargo-registry:/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/cache-target \
    -v raycopy-target:/cache-target \
    raycopy-appimage \
    bash -c 'cargo tauri build --bundles appimage && cp /cache-target/release/bundle/appimage/*.AppImage /app/dist/ && chown -R '"$(id -u):$(id -g)"' /app/dist'

ls -lh dist/*.AppImage
