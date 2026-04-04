<p align="center">
  <img src="branding/moon-icon.svg" width="96" alt="Moon icon">
</p>

# Moon

Moon is a Unity-first scripting language toolkit built around a Rust compiler, a Unity Editor package, and a VS Code extension.

## What is in this repository

- `crates/moonc`: the Moon compiler and CLI
- `unity-package`: the Unity Editor integration and generated-code workflow
- `vscode-moon`: the VS Code extension, syntax assets, snippets, diagnostics, and Unity-aware tooling
- `samples`: sample `.mn` scripts used for local validation and regression checks

## Highlights

- `.mn` source files compile to generated C# for Unity projects
- CLI support for `compile`, `check`, `build`, `init`, `where`, and `version`
- Unity import/build integration with clickable diagnostics back to `.mn` files
- VS Code support for syntax highlighting, snippets, diagnostics, explorer tooling, and generated C# navigation
- Automated BlazeTest EditMode smoke coverage for import, rename, delete, and diagnostic flows

## Quick Start

### Compiler

```powershell
cargo test
cargo run -p moonc -- compile samples\player_controller.mn --output build-output
```

### VS Code extension

```powershell
cd vscode-moon
npm install
npm test
npm run package
```

### Unity smoke verification

```powershell
powershell -ExecutionPolicy Bypass -File .\run-blazetest-smoke.ps1
```

### Full repository verification

```powershell
powershell -ExecutionPolicy Bypass -File .\run-verification.ps1
```

## License

This repository is currently published as `UNLICENSED`. See [LICENSE](LICENSE).
