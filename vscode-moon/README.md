<p align="center">
  <img src="icons/moon-icon.png" width="128" alt="Moon icon">
</p>

# Moon for VS Code

Moon brings Unity-first `.mn` scripting to Visual Studio Code with diagnostics, snippets, script exploration, and generated C# navigation.

## Features

- Syntax highlighting for Moon source files
- Snippets for common Moon declarations and patterns
- `moonc check --json` diagnostics surfaced in the Problems panel
- Explorer tooling for Moon scripts inside the workspace
- Generated C# lookup for Unity and C# Dev Kit workflows
- Unity API-backed completions and script visualization helpers

## Commands

- `Moon: Compile Current File`
- `Moon: Compile Workspace`
- `Moon: Check Current File`
- `Moon: Show Generated C#`
- `Moon: Visualize Script Structure`
- `Moon: Graph View`
- `Moon: Insert Lifecycle Block`

## Settings

- `moon.compilerPath`: explicit path to the `moonc` binary
- `moon.unityApiDbPath`: optional override for the Unity API SQLite database
- `moon.checkOnSave`: run diagnostics on save
- `moon.showWarnings`: control warning visibility in diagnostics

## Packaging

```powershell
npm install
npm test
npm run package
```
