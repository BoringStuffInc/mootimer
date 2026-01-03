# MooTimer - Just commands

# Build release binaries
build:
    cargo build --release

# Install binaries to ~/.local/bin
install: build mcp-install
    mkdir -p ~/.local/bin
    @# Move old binaries aside (works even if running), copy new ones, clean up
    @mv ~/.local/bin/mootimer ~/.local/bin/mootimer.old 2>/dev/null || true
    @cp target/release/mootimer ~/.local/bin/
    @rm ~/.local/bin/mootimer.old 2>/dev/null || true
    @mv ~/.local/bin/mootimerd ~/.local/bin/mootimerd.old 2>/dev/null || true
    @cp target/release/mootimerd ~/.local/bin/
    @rm ~/.local/bin/mootimerd.old 2>/dev/null || true
    @echo "✓ Installation complete (running processes will use new binary on next restart)"
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

# Install MCP daemon to ~/.local/bin and configure daemon client
mcp-install: build
    mkdir -p ~/.local/bin
    @mv ~/.local/bin/mootimerd ~/.local/bin/mootimerd.old 2>/dev/null || true
    @cp target/release/mootimerd ~/.local/bin/
    @rm ~/.local/bin/mootimerd.old 2>/dev/null || true
    @echo "✓ Mootimer MCP daemon installed to ~/.local/bin/mootimerd"
    @echo "  To run as MCP server: ~/.local/bin/mootimerd --mcp"

