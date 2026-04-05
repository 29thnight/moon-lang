# Prism Roslyn Sidecar

This project is the .NET sidecar skeleton for PrSM's Rust LSP.

Current scope:

- stdio JSON-RPC transport with `Content-Length` framing
- protocol models aligned with the Rust sidecar client
- implemented lifecycle methods:
  - `health/ping`
  - `sidecar/initialize`
  - `sidecar/loadProject`
  - `workspace/reload`
  - `sidecar/shutdown`
- implemented query methods against the loaded Roslyn compilation:
  - `unity/completeMembers`
  - `unity/getHover`
  - `unity/getType`
  - `unity/getDefinition`
  - `unity/resolveGeneratedSymbol`

Build:

```powershell
dotnet build
```

Run manually:

```powershell
dotnet run --project .
```

Rust LSP integration is currently opt-in through environment variables:

- `PRISM_ROSLYN_SIDECAR_EXE`
- `PRISM_ROSLYN_SIDECAR_ARGS`

Unity metadata references are discovered from these inputs on the Rust LSP side:

- `.prsmproject` `compiler.target_unity`
- `PRISM_UNITY_MANAGED_DIR`
- `PRISM_UNITY_EDITOR_DIR`
- `Library/ScriptAssemblies/*.dll` under the Unity project root

The next step is improving Unity metadata coverage and documentation quality by feeding richer Unity reference sets and XML docs into the compilation context.