# Development

## Cross-Compilation

Build for all supported platforms from macOS:

```bash
# macOS ARM64 (native)
cargo build --release

# macOS Intel
cargo build --release --target x86_64-apple-darwin

# Linux x86_64
cargo zigbuild --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo zigbuild --release --target aarch64-unknown-linux-gnu
```

Prerequisites for Linux builds:
```bash
brew install zig
cargo install cargo-zigbuild
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
```

## Testing Linux Binaries in Docker

Test the ARM64 Linux binary with the example app:

```bash
docker run --rm -it \
  -v "$(pwd)/target/aarch64-unknown-linux-gnu/release/oit:/usr/local/bin/oit:ro" \
  -v "$(pwd)/example:/app/example:ro" \
  -w /app \
  --platform linux/arm64 \
  ruby:3.3-slim \
  timeout 5 /usr/local/bin/oit -c example/overitall.toml
```

Test x86_64 Linux binary:

```bash
docker run --rm -it \
  -v "$(pwd)/target/x86_64-unknown-linux-gnu/release/oit:/usr/local/bin/oit:ro" \
  -v "$(pwd)/example:/app/example:ro" \
  -w /app \
  --platform linux/amd64 \
  ruby:3.3-slim \
  timeout 5 /usr/local/bin/oit -c example/overitall.toml
```

Quick version check:

```bash
docker run --rm \
  -v "$(pwd)/target/aarch64-unknown-linux-gnu/release/oit:/oit:ro" \
  --platform linux/arm64 \
  debian:bookworm-slim /oit --version
```
