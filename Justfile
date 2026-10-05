default:
    just --list

backend:
    cargo run -p backend

flash:
    cargo run -p firmware --target thumbv8m.main-none-eabihf
