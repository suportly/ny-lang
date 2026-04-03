# Ny Lang — VS Code Extension

Syntax highlighting and Language Server Protocol support for the Ny programming language.

## Features

- **Syntax highlighting** — keywords, types, builtins, strings, f-strings, comments, operators
- **Diagnostics** — inline error reporting from lexer, parser, and type checker
- **Hover** — type information with parameter names for functions, struct fields, enum variants
- **Go-to-definition** — jump to function, struct, and enum definitions
- **Completion** — functions, builtins, structs, enums, keywords, dot-completion for methods
- **Document symbols** — outline view (Ctrl+Shift+O) of all functions, structs, and enums

## Installation

### From source (local install)

```bash
# 1. Build the Ny compiler and LSP server
cd /path/to/ny-lang
cargo build --release

# 2. Make ny-lsp available in PATH
sudo ln -s $(pwd)/target/release/ny-lsp /usr/local/bin/ny-lsp

# 3. Install the extension in VS Code
cd editors/vscode
npm install
```

Then in VS Code:
1. Open Command Palette (Ctrl+Shift+P)
2. Run "Developer: Install Extension from Location..."
3. Select the `editors/vscode` directory

### Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `ny.lspPath` | `ny-lsp` | Path to the ny-lsp binary |

## Requirements

- [Ny compiler](../../README.md#installation) built with `cargo build --release`
- `ny-lsp` binary in PATH (or configured via `ny.lspPath`)
