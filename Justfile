default:
    just --list

flash:
    cd firmware && cargo run

sim:
    cargo run -p sim
sim-dev:
    watchexec --restart --watch sim --watch common --watch ui --exts rs,toml -- cargo run -p sim

server:
    cargo run -p server

dev:
    watchexec --restart --watch server --watch common --exts rs,toml,html,css -- cargo run -p server --features livereload

hub:
    cargo run -p hub

build-firmware:
    cd firmware && cargo build

build-server:
    cargo build -p server

build-hub:
    cargo build -p hub

build:
    cargo build --workspace
