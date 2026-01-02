# MooTimer - Just commands

# Build release binaries
build:
    cargo build --release

# Install binaries to ~/.local/bin
install: build
    mkdir -p ~/.local/bin
    @# Try to copy binaries, skip if busy (already running)
    @cp target/release/mootimer ~/.local/bin/ 2>/dev/null || echo "⚠ TUI already running, skipped mootimer (close TUI to update)"
    @cp target/release/mootimerd ~/.local/bin/ 2>/dev/null || echo "⚠ Daemon already running, skipped mootimerd (restart daemon to update)"
    @echo "✓ Installation complete"
    @which mootimer
    @which mootimerd

# Run the TUI
run:
    cargo run --bin mootimer

# Run the daemon
daemon:
    cargo run --bin mootimerd

# Run tests
test:
    cargo test

# Clean build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Build and run the TUI
dev: build run
