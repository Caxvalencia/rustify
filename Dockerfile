# Imagen de Rust oficial que soporta la edición 2024 (Rust 1.85+)
FROM rust:1.85-bookworm AS builder

# Instalar Node.js (versión 20) y pnpm para herramientas de Node.js en packages/
RUN apt-get update && apt-get install -y curl gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g pnpm && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copiar archivos de configuración del proyecto
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY packages/ packages/

# Construir el compilador en modo release por defecto en la imagen
RUN cargo build --release

# Etapa de runtime para una imagen pequeña y lista para producción
FROM debian:bookworm-slim

WORKDIR /app

# Instalar Node.js en la imagen final para soportar el modo híbrido y las herramientas
RUN apt-get update && apt-get install -y curl gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Copiar el binario compilado desde el builder
COPY --from=builder /workspace/target/release/rustify-cli /usr/local/bin/rustify

# Confirmar la instalación de rustify
ENTRYPOINT ["rustify"]
