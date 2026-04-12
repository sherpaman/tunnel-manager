# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands
- Build: `cargo build`
- Run: `cargo run -- [args]`
- Test: `cargo test`
- Test single: `cargo test [module]::[test_name]`

## Architecture
- `src/config/mod.rs`: Parses `~/.ssh/config` to extract tunnel definitions (Host + LocalForward/RemoteForward).
- `src/manager/mod.rs`: Manages the lifecycle of SSH tunnel processes (spawn, kill, list).
- `src/main.rs`: CLI entry point using `clap` for command parsing.
