#
# AI HAS BEEN USED TO GENERATE THIS DOCKERFILE, I STRUGGLE WITH DOCKER
#

# ---------------------------------------------------------------- build
FROM rust:1-trixie AS builder

RUN apt-get update -y \
 && apt-get install -y --no-install-recommends brotli \
 && rm -rf /var/lib/apt/lists/* \
 && rustup target add wasm32-unknown-unknown \
 && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
 && cargo binstall -y cargo-leptos

WORKDIR /work
COPY . .

# `ocr-bundled` links ONNX Runtime statically; use `--bin-features ssr` for a
# build without image scanning.
ARG FEATURES=ocr-bundled
RUN LEPTOS_HASH_FILES=true cargo leptos build --release --bin-features "$FEATURES" \
 && find target/site -type f \( -name '*.js' -o -name '*.wasm' -o -name '*.css' -o -name '*.webmanifest' \) \
      -exec brotli -kq 11 {} \; -exec gzip -k9 {} \;

# ------------------------------------------------------- dictionary data
# The original, unmodified distribution files. Rebuild the image to update them.
FROM debian:trixie-slim AS data
RUN apt-get update -y \
 && apt-get install -y --no-install-recommends curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /data
# The OCR models are pinned to a release, so this layer can stay cached.
RUN mkdir ocr_models \
 && for f in pp-ocrv5_mobile_det.onnx pp-ocrv5_mobile_rec.onnx ppocrv5_dict.txt; do \
      curl -fsSL -o "ocr_models/$f" "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0/$f"; \
    done
# The dictionaries are updated upstream all the time. Pass a new value (the CI
# workflow uses its run id) to download them again while every other layer,
# including the slow Rust build, still comes from the cache.
ARG DATA_VERSION=unset
RUN echo "dictionary data: $DATA_VERSION" \
 && curl -fsSL http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz | gunzip > JMdict_e \
 && curl -fsSL https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz | gunzip > cedict_ts.u8 \
 && grep -q '<JMdict>' JMdict_e \
 && grep -q '^[^#].* \[.*\] /' cedict_ts.u8

# ------------------------------------------------------------------ run
FROM debian:trixie-slim

RUN apt-get update -y \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 delphinus

WORKDIR /app
COPY --from=builder /work/target/release/delphinus /work/target/release/hash.txt /app/
COPY --from=builder /work/target/site /app/site
COPY --from=data /data/JMdict_e /data/cedict_ts.u8 /app/dictionaries/
COPY --from=data /data/ocr_models /app/ocr_models

ENV LEPTOS_OUTPUT_NAME=delphinus \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_HASH_FILES=true \
    LEPTOS_ENV=PROD \
    LEPTOS_SITE_ADDR=0.0.0.0:8080 \
    DELPHINUS_THREADS=2 \
    DELPHINUS_OCR_IDLE_SECS=300 \
    DISABLE_OCR=false \
    # Fewer glibc malloc arenas: less memory held per thread.
    MALLOC_ARENA_MAX=2

USER delphinus
EXPOSE 8080
HEALTHCHECK --interval=60s --timeout=3s CMD ["/bin/bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080 && printf 'GET /healthz HTTP/1.0\\r\\n\\r\\n' >&3 && grep -q ok <&3"]
CMD ["/app/delphinus"]
