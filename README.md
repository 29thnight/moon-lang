<p align="center">
  <img src="branding/prsm-logo-white.svg" width="96" alt="PrSM icon">
</p>

# PrSM

PrSM is a Unity-first scripting language toolkit built around a Rust compiler, a Unity Editor package, and a VS Code extension.

## What is in this repository

- `crates/refraction`: the PrSM compiler and CLI
- `roslyn-sidecar`: an optional .NET Roslyn sidecar skeleton for Unity/C# symbol queries
- `unity-package`: the Unity Editor integration and generated-code workflow
- `vscode-prsm`: the VS Code extension, syntax assets, snippets, diagnostics, and Unity-aware tooling
- `samples`: sample `.prsm` scripts used for local validation and regression checks

## Highlights

- `.prsm` source files compile to generated C# for Unity projects
- compile/build also emit sidecar `.prsmmap.json` files next to generated C# for VS Code navigation and Unity generated-script back-mapping
- CLI support for `compile`, `check`, `build`, `hir`, `definition`, `references`, `index`, `lsp`, `init`, `where`, and `version`
- Unity import/build integration with clickable diagnostics, remapped generated-script runtime stack traces, and generated `.cs` redirect support back to PrSM source
- VS Code support for syntax highlighting, snippets, diagnostics, trusted-workspace `prism lsp` navigation, explorer tooling, and generated C# hover/navigation
- Automated BlazeTest EditMode smoke coverage for import, rename, delete, and diagnostic flows
- In-repo Unity package EditMode tests for project config, diagnostic formatting, compiler resolution, and source-map helpers

## Quick Start

### Compiler

```powershell
cargo test
cargo run -p refraction --bin prism -- compile samples\PlayerController.prsm --output build-output
cargo run -p refraction --bin prism -- hir . --json
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- index . --json
cargo run -p refraction --bin prism -- index . --json --file samples\PlayerController.prsm --line 10 --col 5
```

### VS Code extension

```powershell
cd vscode-prsm
npm install
npm test
npm run package
```

### Roslyn sidecar skeleton

```powershell
cd roslyn-sidecar
dotnet build
```

The Rust LSP can launch it when `PRISM_ROSLYN_SIDECAR_EXE` is set. Unity metadata references are discovered from `.prsmproject` `compiler.target_unity`, `PRISM_UNITY_MANAGED_DIR`, `PRISM_UNITY_EDITOR_DIR`, and `Library/ScriptAssemblies` when available.

### Unity smoke verification

```powershell
powershell -ExecutionPolicy Bypass -File .\run-blazetest-smoke.ps1
```

### Full repository verification

```powershell
powershell -ExecutionPolicy Bypass -File .\run-verification.ps1
```

The repository verification flow now includes VS Code extension packaging, bundled artifact verification, and an isolated VSIX install smoke.

## License

This repository is currently published as `UNLICENSED`. See [LICENSE](LICENSE).
