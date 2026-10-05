default:
    just --list

flash:
    cargo run -p firmware --target thumbv8m.main-none-eabihf

server:
    cargo run -p server

dev:
    watchexec --restart --watch server --watch common --exts rs,toml,html,css -- cargo run -p server --features livereload

hub:
    cargo run -p hub

build-firmware:
    cargo build -p firmware --target thumbv8m.main-none-eabihf

build-server:
    cargo build -p server

build-hub:
    cargo build -p hub

build:
    cargo build --workspace
