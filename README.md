# tree-rs

A fast, cross-platform command-line tool written in Rust that displays directory structures in a tree-like format — similar to the Unix `tree` command.

## Features

- Recursive directory traversal with configurable depth
- Filter files by glob pattern (include or exclude)
- Show or hide hidden files (dotfiles)
- Display only directories
- Show full absolute paths
- Summary of directory and file counts

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)

### Build from source

```bash
git clone https://github.com/yalamaddisasiKumar/tree-rs.git
cd tree-rs
cargo build --release
```

The binary will be at `target/release/tree-rs` (or `tree-rs.exe` on Windows).

### Run directly

```bash
cargo run -- [PATH] [OPTIONS]
```

## Usage

```
tree-rs [PATH] [OPTIONS]
```

`PATH` defaults to the current directory (`.`) if not specified.

## Options

| Flag | Description |
|------|-------------|
| `-a` | Show all files, including hidden ones (dotfiles) |
| `-d` | Show only directories |
| `-L <N>` | Limit display depth to `N` levels |
| `-f` | Show full absolute paths instead of names |
| `-P <PATTERN>` | Include only files matching the glob pattern (e.g. `*.rs`) |
| `-I <PATTERN>` | Exclude files matching the glob pattern (e.g. `*.lock`) |

## Examples

Display the current directory:
```bash
tree-rs
```

Display a specific path up to 2 levels deep:
```bash
tree-rs /path/to/project -L 2
```

Show only Rust source files:
```bash
tree-rs src -P "*.rs"
```

Show all files including hidden, excluding the `target` directory:
```bash
tree-rs -a -I "target"
```

Show only directories up to 3 levels deep:
```bash
tree-rs -d -L 3
```

Show full paths for all `.toml` files:
```bash
tree-rs -f -P "*.toml"
```

## Sample Output

```
tree-rs
├── Cargo.lock
├── Cargo.toml
├── README.md
└── src
    └── main.rs

1 directories, 4 files
```

## Dependencies

- [`clap`](https://crates.io/crates/clap) — Argument parsing
- [`glob`](https://crates.io/crates/glob) — Pattern matching for `-P` and `-I` filters

## License

MIT
