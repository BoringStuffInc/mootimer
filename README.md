```
.....................................................................
......----..:..:-:-----=**++=-+=:----:::..............:.:.........:-.
......----:.:..::-:.::::==*@@@@@@@@@@@@%=.............:.:.......::-=.
...:..----:.:..::--.::::=@@@@@@@@@@@@@@@@#............:.:.......:.--.
..::.:----.:....:--:::-%@@@+::======-=*@@@@-............:.::....::-:.
.----=--:-::--:.::--.+@@@@---:-:==-:=-:-#@@@*-...--=#@@@@@*=--:::---.
.%%#%##%@@@@@@@@@@@@@@@@-=::--.:=:-===.:-=@@@@@@@@@@@@@@@@@@@*=--+*+.
.@@@@@@@#-::+%@@@@@@@@=+=:..:===-:-=-+===+-%@@@@@@@@%=@@@@==@@@@@@@%.
.@@@@@@+@@@@@@@%%@@#+%@*=---:.:-----------==++#@@@@%#%@@@@@@*@@@@@%#.
.%@@@@@@@+=+#-+*%@@@@@@#++**+---:-:-.::-.:---#%@@@@#+++*-=#@@-==----.
.::.:-%@@@@#=:=+%%######+*++**+--:-----===-::#@%%##+===%@@@@=--::---.
..:..:-*@@@@=.-=+-::.=##*@@%*#%*=::-----===#@@%@=-+#:-=+@%=---:::..:.
.----====-%@@@@###%@@#+##*=*++*+=-::====.--@%%=@@%+#+*=*-----:::::.:.
.=+=--::-:-#@@@@%@@@@@+=-%@#+++====--:+=---@@@=##%%#=-==-------:--::.
.-::::--:.-+*++#*+===##+=-=*++-:-===-.=++-:==:-::-=-.--:::.:.:..::--.
.-:-=+++==++==----==-+==-.:::-====-=-:::=---=-:::::::.::-::::::::.::.
.-==++=-====-:--=--=-+:----:::==#---=-.:-=--=::::::-------::...::---.
.----------:::-:-=+=-+::-::---=++-+-:=--:====::::.:::::::.::::::-:::.
.---=:-:::::::-:----=+--:::::::==:#@#*+=++++-::::::.::::--::::::::.:.
.=--=:-:--::----:-::-++--------=:%@@@@%@@@@=--:.::..::..:.::..:...:-.
.-::--:-::---===-::--++-------=+-%%=*@@@%=#+---:--:::::::::.......::.
..::--:-----====+::--+==----:-+=#%**-*#**+#+---:::::.::..::::::.::.:.
.---:.:==:.--=-=*+--+=-==-=+=-=+**++=:-=+#*=---:.--:::::::-:::::::...
..:--:-==-:--=:-==-=+----*=+#*==-+**++*#@@@+:---:-:.:.:....:.::...::.
..-::----:-=-:-++:-++=+==#=-=*#+=:=*#%%@@@=*-:.:-::::::..:-::::::-==.
..:------==-:-==:-+%##*=++--::=+----+#@@%-+*------::....::..::-====+.
.::--:.:=--:--==+=#%++=:---::-:==---=++===#=.:-:::::-----:.:-===++++.
.-----:-=::----==+#*.-=:=-:.::--=--+==++=+#-::.:-::..::..:=++++==+#%.
.--::-==---::-=-==+=::==---.:..-=-=+=-+==**-:.:::--:----=+*++===+**+.
.....................................................................
                                    M O O !
```

# MooTimer

A terminal-based work timer system with profile management, Pomodoro support, and Git-based data synchronization.

## Overview

MooTimer uses a **daemon-client** architecture:
- `mootimerd`: Background service managing state, persistence, and synchronization.
- `mootimer`: Ratatui-based TUI frontend for interaction.

## Key Features

- **Time Tracking**: Manual, Pomodoro, and Countdown timer modes.
- **Profiles**: Separate contexts for different companies or personal projects.
- **Git Sync**: Automatic versioning and multi-device synchronization of your time logs.
- **MCP Support**: Built-in Model Context Protocol server for LLM integration.

## Installation

### Prerequisites
- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install) (latest stable)
- **just** (optional): [Install Just](https://github.com/casey/just) for easier command execution

### Easy Install (recommended)
If you have `just` installed, simply run:
```bash
just install
```
This will build the binaries in release mode and install them to `~/.local/bin/`.

### Manual Installation
If you don't have `just`, you can build and install manually:
```bash
cargo build --release
mkdir -p ~/.local/bin
cp target/release/mootimer* ~/.local/bin/
```

## Quick Start

1. **Start the daemon**:
   ```bash
   mootimerd
   ```
   (Or run in background: `mootimerd &`)

2. **Start the TUI**:
   ```bash
   mootimer
   ```

## Data Storage
Logs and configuration are stored in `~/.mootimer/`. If initialized as a Git repository, the daemon handles automatic commits and synchronization.

## License
MIT
