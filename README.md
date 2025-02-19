# carve

`carve` is a terminal-focused tool for interactively searching and exploring text files and logs. It provides a user-friendly interface for real-time text analysis with features like live tailing, searching, and filtering.

> **Note:** This project is currently experimental. It only supports Unix-like systems and it's performance may be underwhelming on anything but trivial log sizes at this time. Please report any issues or bugs on GitHub: [https://github.com/deej-io/carve](https://github.com/deej-io/carve)

## Features


- Interactive TUI Terminal User Interface
- Real-time file tailing
- Live search with match highlighting
- Filter expressions
- Multiple operation modes:
  - Normal mode: Manual scrolling and navigation
  - Search mode: Interactive text search
  - Filter mode: Apply filters to displayed content

## Installation

To install `carve`, you'll need Rust and Cargo installed on your system. Then run:

```bash
cargo install carve
```

Or build from source:

```bash
git clone https://github.com/deej-io/carve
cd carve
cargo build --release
```

## Usage

Basic usage:

```bash
# Pipe command output into carve, interactively searching and filtering it before storing the result
node serve.js | carve > filtered-log.txt
```

### Keyboard Controls

- Normal Mode:
  - Arrow keys / j/k: Scroll up/down
  - `/`: Enter filter mode
  - `f`: Enter searc mode
  - `q`: Quit

- Search Mode:
  - Enter: Execute search
  - `n`: Next match
  - `N`: Previous match
  - Esc: Return to normal mode

- Filter Mode:
  - Enter: Apply filter
  - Esc: Return to normal mode

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.

## Author

Daniel J Rollins <me@deej.io>

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
