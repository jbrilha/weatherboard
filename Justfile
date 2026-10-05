default:
    just --list

flash:
    cargo run -p firmware --target thumbv8m.main-none-eabihf

backend:
    cargo run -p backend

hub:
    cargo run -p hub

build-firmware:
    cargo build -p firmware

build-backend:
    cargo build -p backend

build-hub:
    cargo build -p hub

build:
    cargo build --workspace
