FROM ubuntu:24.04 as builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    curl build-essential npm cmake git ca-certificates pkg-config libssl-dev wget && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"

# Download latest Binaryen release from GitHub and install it
RUN BINARYEN_VERSION=$(curl -s https://api.github.com/repos/WebAssembly/binaryen/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/') \
 && wget https://github.com/WebAssembly/binaryen/releases/download/${BINARYEN_VERSION}/binaryen-${BINARYEN_VERSION}-x86_64-linux.tar.gz \
 && tar -xzf binaryen-${BINARYEN_VERSION}-x86_64-linux.tar.gz \
 && cp -r binaryen-${BINARYEN_VERSION}/bin/* /usr/local/bin/ \
 && rm -rf binaryen-${BINARYEN_VERSION} binaryen-${BINARYEN_VERSION}-x86_64-linux.tar.gz

RUN npm install -g sass

RUN curl --proto '=https' --tlsv1.3 -LsSf https://github.com/leptos-rs/cargo-leptos/releases/latest/download/cargo-leptos-installer.sh | sh

RUN rustup target add wasm32-unknown-unknown

WORKDIR /work
COPY . .

RUN npm install

RUN RUSTFLAGS="--cfg erase_components" cargo leptos build --release -vv

FROM ubuntu:24.04 as runner

WORKDIR /app

RUN apt-get update -y \
 && apt-get install -y --no-install-recommends openssl ca-certificates curl \
 && apt-get autoremove -y \
 && apt-get clean -y \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /work/target/release/delphinus /app/
COPY --from=builder /work/target/site /app/site
COPY --from=builder /work/Cargo.toml /app/
COPY --from=builder /work/dictionaries /app/dictionaries

RUN mkdir -p /app/ocr_models && \
 curl -L -o /app/ocr_models/ppocrv5_mobile_det.onnx \
 https://github.com/GreatV/oar-ocr/releases/download/v0.1.0/ppocrv5_mobile_det.onnx && \
 curl -L -o /app/ocr_models/ppocrv5_mobile_rec.onnx \
 https://github.com/GreatV/oar-ocr/releases/download/v0.1.0/ppocrv5_mobile_rec.onnx && \
 curl -L -o /app/ocr_models/ppocrv5_dict.txt \
 https://github.com/GreatV/oar-ocr/releases/download/v0.1.0/ppocrv5_dict.txt

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT=./site
ENV DISABLE_ORC="FALSE"

EXPOSE 8080
CMD ["/app/delphinus"]