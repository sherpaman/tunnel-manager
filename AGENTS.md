# Agent Instructions

## Development Commands
- Build: `cargo build` (use `--release` for production binaries)
- Run TUI: `cargo run -- tui`
- Run CLI: `cargo run -- [args]`
- Test: `cargo test`
- Test single: `cargo test [module]::[test_name]`

## Architecture & Behavior
- **SSH Config Dependency**: Relies on `~/.ssh/config` to define tunnels (Host + LocalForward/RemoteForward).
- **Process Management**: `src/manager/mod.rs` handles the lifecycle (spawn, kill, list) of SSH tunnel processes.
- **TUI**: Interactive management is available via the `tui` subcommand.
