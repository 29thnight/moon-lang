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
- compile/build also emit sidecar `.mnmap.json` files next to generated C# for VS Code navigation and Unity generated-script back-mapping
- CLI support for `compile`, `check`, `build`, `hir`, `definition`, `references`, `index`, `init`, `where`, and `version`
- Unity import/build integration with clickable diagnostics, remapped generated-script runtime stack traces, and generated `.cs` redirect support back to Moon source
- VS Code support for syntax highlighting, snippets, diagnostics, explorer tooling, document/workspace symbols, references/rename, and generated C# navigation
- Automated BlazeTest EditMode smoke coverage for import, rename, delete, and diagnostic flows
- In-repo Unity package EditMode tests for project config, diagnostic formatting, compiler resolution, and source-map helpers

## Quick Start

### Compiler

```powershell
cargo test
cargo run -p moonc -- compile samples\player_controller.mn --output build-output
cargo run -p moonc -- hir . --json
cargo run -p moonc -- definition . --json --file samples\PlayerController.mn --line 10 --col 5
cargo run -p moonc -- references . --json --file samples\PlayerController.mn --line 10 --col 5
cargo run -p moonc -- index . --json
cargo run -p moonc -- index . --json --file samples\PlayerController.mn --line 10 --col 5
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
