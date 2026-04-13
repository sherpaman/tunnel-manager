# Tunnel Manager

A command-line tool for managing SSH tunnels, written in Rust.

## Features
- Parses your `~/.ssh/config` to extract tunnel definitions (Host + LocalForward/RemoteForward)
- Manages the lifecycle of SSH tunnel processes (spawn, kill, list)
- TUI (Terminal User Interface) for interactive management
- CLI interface using `clap`

## Getting Started

### Prerequisites
- Rust (https://rustup.rs/)
- SSH client installed

### Build
```
cargo build --release
```

### Run
```
cargo run -- [args]
```

### Test
```
cargo test
```

## Usage

- Start the CLI:
  ```
  cargo run --
  ```
- Use the TUI for interactive management (if available):
  ```
  cargo run -- tui
  ```
- List, start, and stop tunnels as defined in your `~/.ssh/config`.

## Project Structure
- `src/config/mod.rs`: Parses SSH config for tunnel definitions
- `src/manager/mod.rs`: Manages SSH tunnel processes
- `src/main.rs`: CLI entry point
- `src/tui.rs`: Terminal UI (if implemented)

## License
MIT
