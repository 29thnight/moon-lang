# PrSM Component Map for Triage

## Core surfaces

- `crates/refraction`: compiler, CLI, LSP server, source-map emission
- `vscode-prsm`: VS Code client, hover/definition/references, packaged binary selection, VSIX packaging
- `unity-package`: Unity editor integration, generated-source redirect, stack-trace remap, source-map consumption
- `run-blazetest-smoke.ps1`: external Unity smoke entrypoint for BlazeTest
- `run-verification.ps1`: repository-wide Rust + VS Code + BlazeTest verification

## Fast heuristics

- CLI output wrong, parse/semantic/codegen wrong: start at `crates/refraction`
- Editor hover, rename, workspace symbol, install smoke wrong: start at `vscode-prsm`
- Runtime stack trace, generated `.cs` redirection, Unity import or rename issues: start at `unity-package`
- Works in repo but not in BlazeTest: inspect external project assumptions, compiler path resolution, project-root handling, or stale generated artifacts
- Generated C# and original `.prsm` mismatch: treat as cross-surface and involve boundary QA early