# MooTimer

A flexible work timer application with profile support, JIRA integration, pomodoro functionality, and git-based synchronization.

## Features

- **Time Logging** - Track work sessions with start/stop/pause
- **Profile Management** - Multiple profiles (company A, company B, personal, etc.)
- **JIRA Integration** - Import tasks from JIRA
- **Pomodoro Timer** - Customizable intervals with task tracking
- **Git-based Sync** - Export/sync via text files through git
- **Multiple Frontends** - Daemon + frontend architecture (TUI first, GUI/Web later)

## Architecture

MooTimer uses a daemon + frontend architecture:

- **mootimerd** - Background daemon that manages timers and data
- **mootimer** - TUI (Terminal User Interface) frontend
- Data stored in `~/.mootimer/` with git support for synchronization

## Quick Start

### Prerequisites

- Rust 1.75 or later
- Git (for synchronization features)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/mootimer
cd mootimer

# Build all components
cargo build --release

# Binaries will be in target/release/
# - mootimerd (daemon)
# - mootimer (TUI)
```

### Running

```bash
# Start the daemon
./target/release/mootimerd

# In another terminal, start the TUI
./target/release/mootimer
```

### Installation

```bash
# Build and install
cargo install --path crates/mootimer-daemon
cargo install --path crates/mootimer-tui

# The binaries will be installed to ~/.cargo/bin/
# Make sure ~/.cargo/bin is in your PATH
```

## Development

### Project Structure

```
mootimer/
├── crates/
│   ├── mootimer-core/      # Core data models and storage
│   ├── mootimer-daemon/    # Daemon process (mootimerd)
│   ├── mootimer-client/    # Client library for IPC
│   ├── mootimer-tui/       # TUI frontend (mootimer)
│   └── mootimer-jira/      # JIRA integration
├── docs/                   # Documentation
├── scripts/                # Installation and setup scripts
└── Cargo.toml             # Workspace definition
```

### Building

```bash
# Build all crates
cargo build

# Build specific crate
cargo build -p mootimer-core

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin mootimerd
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p mootimer-core

# Run with output
cargo test -- --nocapture
```

## Configuration

MooTimer stores its data in `~/.mootimer/`:

```
~/.mootimer/
├── config.json           # Global configuration
├── profiles/             # Profile directories
│   ├── company_a/
│   │   ├── profile.json
│   │   ├── tasks.json
│   │   └── entries.csv
│   └── personal/
│       ├── profile.json
│       ├── tasks.json
│       └── entries.csv
└── .git/                 # Git repository (if initialized)
```

### Example config.json

```json
{
  "version": "1.0.0",
  "default_profile": "personal",
  "daemon": {
    "socket_path": "/tmp/mootimer.sock",
    "log_level": "info"
  },
  "pomodoro": {
    "work_duration": 1500,
    "short_break": 300,
    "long_break": 900,
    "sessions_until_long_break": 4
  },
  "sync": {
    "auto_commit": true,
    "auto_push": false,
    "remote_url": null
  }
}
```

## Usage

### Basic Time Tracking

1. Start the daemon: `mootimerd`
2. Open the TUI: `mootimer`
3. Create a profile (or use the default)
4. Create or import tasks
5. Start a timer
6. Work!
7. Stop the timer when done

### JIRA Integration

1. Configure JIRA in profile settings
2. Enter JIRA URL and credentials
3. Import tasks using JQL query
4. Tasks will be synced to your profile

### Git Synchronization

1. Initialize git in data directory:
   ```bash
   cd ~/.mootimer
   git init
   git remote add origin <your-repo>
   ```
2. Enable auto-sync in config
3. MooTimer will auto-commit changes
4. Optionally enable auto-push for multi-device sync

## Documentation

- [Architecture](ARCHITECTURE.md) - Detailed technical architecture
- [Roadmap](ROADMAP.md) - Implementation plan and timeline
- [User Guide](docs/user-guide.md) - Complete user documentation (coming soon)
- [API Documentation](docs/api.md) - JSON-RPC API reference (coming soon)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Run formatter: `cargo fmt`
6. Run linter: `cargo clippy`
7. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details

## Status

🚧 **Work in Progress** - MooTimer is currently under active development.

Current progress:
- [x] Project setup and structure
- [ ] Core data models
- [ ] Storage layer
- [ ] Timer engine
- [ ] Daemon IPC server
- [ ] TUI frontend
- [ ] Pomodoro timer
- [ ] JIRA integration
- [ ] Git synchronization

See [ROADMAP.md](ROADMAP.md) for detailed implementation plan.
