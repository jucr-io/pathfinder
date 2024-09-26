# Stable rust 1.x on ARM64
FROM mcr.microsoft.com/devcontainers/rust:1-bullseye

# Additional nightly toolchain
RUN rustup toolchain install nightly

# Rust Components
RUN rustup component add --toolchain nightly rustfmt
RUN rustup component add clippy rustfmt
RUN cargo install cargo-expand

# Always use https for git as we're in a codespace
RUN git config --global url."https://github.com/".insteadOf ssh://git@github.com/
