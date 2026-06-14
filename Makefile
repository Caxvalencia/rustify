.PHONY: build-dev build-release podman-build docker-build test clean help

# Comportamiento por defecto
.DEFAULT_GOAL := help

help:
	@echo "Comandos disponibles en Rustify:"
	@echo "  make build-dev      - Compila el proyecto en modo desarrollo (debug)"
	@echo "  make build-release  - Compila el proyecto optimizado en modo release"
	@echo "  make podman-build   - Construye la imagen de contenedor de Rustify usando Podman"
	@echo "  make docker-build   - Construye la imagen de contenedor de Rustify usando Docker"
	@echo "  make test           - Ejecuta los tests del compilador"
	@echo "  make clean          - Limpia los archivos compilados de Cargo"

# Build en modo desarrollo (debug)
build-dev:
	cargo build

# Build en modo release
build-release:
	cargo build --release

# Construir contenedor con Podman
podman-build:
	podman build -t rustify:latest .

docker-build:
	docker build -t rustify:latest .

# Ejecutar tests
test:
	cargo test

# Limpiar compilación
clean:
	cargo clean
