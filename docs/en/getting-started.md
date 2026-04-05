---
title: Getting Started
parent: Introduction
nav_order: 2
---

# Getting Started

## Compiler workflow

```powershell
cargo test
cargo run -p refraction --bin prism -- compile samples\PlayerController.prsm --output build-output
cargo run -p refraction --bin prism -- check samples\PlayerController.prsm
```

## Initialize a project

```powershell
cargo run -p refraction --bin prism -- init
```

That creates a `.prsmproject` file with project metadata, source globs, and compiler output settings.

## Explore analysis commands

```powershell
cargo run -p refraction --bin prism -- hir . --json
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- index . --json
```

## Validate editor tooling

```powershell
cd vscode-prsm
npm install
npm test
```
