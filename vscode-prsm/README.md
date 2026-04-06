<p align="center">
  <img src="icons/prsm-logo.png" width="128" alt="PrSM icon">
</p>

# PrSM for VS Code

PrSM brings Unity-first `.prsm` scripting to Visual Studio Code with diagnostics, snippets, script exploration, and generated C# navigation.

## Features

- Syntax highlighting for PrSM source files
- Snippets for common PrSM declarations and patterns
- `prism check --json` diagnostics surfaced in the Problems panel
- Explorer tooling for PrSM scripts inside the workspace
- Generated C# lookup for Unity and C# Dev Kit workflows
- Unity API-backed completions and script visualization helpers

## Commands

- `PrSM: Compile Current File`
- `PrSM: Compile Workspace`
- `PrSM: Check Current File`
- `PrSM: Show Generated C#`
- `PrSM: Visualize Script Structure`
- `PrSM: Graph View`
- `PrSM: Insert Lifecycle Block`

## Settings

- `prsm.compilerPath`: explicit path to the `prism` binary
- `prsm.unityApiDbPath`: optional override for the Unity API SQLite database
- `prsm.checkOnSave`: run diagnostics on save
- `prsm.showWarnings`: control warning visibility in diagnostics

## Packaging

```powershell
npm install
npm test
npm run package
npm run verify
npm run verify:install
```

`npm run package` now writes a VSIX under `artifacts/` and verifies that the packaged `dist/extension.js` and `bin/prism.exe` exactly match the current workspace build outputs.
`npm run verify` checks bundled `bin/prism.exe` freshness and CLI behavior, while `npm run verify:install` installs the generated VSIX into an isolated VS Code profile and verifies the extracted manifest, bundle, and bundled compiler.
