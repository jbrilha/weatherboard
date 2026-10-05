default:
    just --list

flash:
    cargo run -p firmware --target thumbv8m.main-none-eabihf

server:
    cargo run -p server

hub:
    cargo run -p hub

build-firmware:
    cargo build -p firmware

build-server:
    cargo build -p server

build-hub:
    cargo build -p hub

build:
    cargo build --workspace
