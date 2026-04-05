---
title: Getting Started
parent: Introduction
nav_order: 2
---

# Getting Started

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)
- Node.js 18+ and npm (for the VS Code extension)
- Unity 2021.3 LTS or later (for the Unity package)
- VS Code with the PrSM extension (optional but recommended)

## 1. Build the compiler

```powershell
cargo build -p refraction
```

Or run tests to confirm the build is clean:

```powershell
cargo test
```

## 2. Compile a sample file

```powershell
cargo run -p refraction --bin prism -- compile samples\PlayerController.prsm --output build-output
```

The `build-output` directory will contain the generated `.cs` file and a `.prsmmap.json` sidecar.

## 3. Check without generating output

```powershell
cargo run -p refraction --bin prism -- check samples\PlayerController.prsm
```

Use `--json` for machine-readable diagnostics.

## 4. Initialize a new project

```powershell
cargo run -p refraction --bin prism -- init
```

This creates a `.prsmproject` file in the current directory. Edit it to configure source glob patterns, the output directory, and target Unity version.

## 5. Build a full project

```powershell
cargo run -p refraction --bin prism -- build
```

Use `--watch` for continuous compilation during development:

```powershell
cargo run -p refraction --bin prism -- build --watch
```

## 6. Explore analysis commands

These commands drive the VS Code extension's navigation features:

```powershell
# Dump the Typed HIR
cargo run -p refraction --bin prism -- hir . --json

# Resolve the definition at a file position
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5

# Find all references to the symbol at a position
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5

# Query the project symbol index by name
cargo run -p refraction --bin prism -- index . --json --symbol PlayerController
```

## 7. VS Code extension

```powershell
cd vscode-prsm
npm install
npm test          # run extension tests
npm run package   # build and verify a .vsix
```

Install the generated `.vsix` from `vscode-prsm/artifacts/` via **Extensions > Install from VSIX** in VS Code.
