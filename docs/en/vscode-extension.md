---
title: VS Code Extension
parent: Tooling
grand_parent: English Docs
nav_order: 3
---

# VS Code Extension

The PrSM VS Code extension (`prsm-lang`) provides a full editing workflow for `.prsm` files: syntax highlighting, real-time diagnostics, navigation, code actions, and visual tools for inspecting script structure and generated C#.

## Installation

Install from the VS Code Marketplace by searching **PrSM**, or from a `.vsix` file:

1. Download `parkyoungung.prsm-lang-x.x.x.vsix` from [GitHub Releases](https://github.com/29thnight/PrSM/releases)
2. In VS Code: **Extensions > ··· > Install from VSIX**

The MSI installer also installs the extension automatically if VS Code is detected.

## Language Features (via LSP)

When the workspace is **trusted**, the extension connects to the `prism lsp` language server and provides:

- **Real-time diagnostics** — errors and warnings as you type, powered by the PrSM semantic analyzer
- **Go-to-definition** (F12 or Ctrl+Click) — jump to symbol declarations, including cross-file navigation
- **Find all references** (Shift+F12) — find every usage of a symbol across the project
- **Hover information** — type info, documentation, and generated C# details on mouse hover
- **Rename symbol** (F2) — rename a symbol and all its references across the project
- **Document symbols** (Ctrl+Shift+O) — outline of all declarations in the current file
- **Workspace symbols** (Ctrl+T) — search symbols across the entire project
- **Code actions** — explicit generic type argument insertion, organize imports
- **Completion** — Unity API (SQLite DB), user-defined symbols, and keywords

## Editor Features

### Syntax Highlighting

55 TextMate scopes covering keywords, types, operators, strings, comments, annotations, and PrSM-specific constructs like `listen`, `require`, `coroutine`.

### Code Snippets

20+ snippets for common patterns:

| Prefix | Inserts |
|--------|---------|
| `comp` | Component declaration scaffold |
| `asset` | ScriptableObject asset declaration |
| `func` | Function with parameters |
| `cor` | Coroutine with wait |
| `listen` | Listen block |
| `if` | If/else block |
| `when` | When pattern match |
| `for` | For loop with range |
| `ser` | Serialize field |
| `req` | Require field |

Type the prefix and press Tab to expand.

### PrSM Explorer

A sidebar tree view showing all `.prsm` files in the workspace. Click to open, refresh button at the top.

### Graph View

Visualizes component dependency relationships as an interactive graph. Shows which components reference each other through `require`, `optional`, `child`, and `parent` fields.

Open via: **Ctrl+Shift+V** or Command Palette: `PrSM: Graph View`

### Visualize Script Structure

Displays the internal structure of the current `.prsm` file — declarations, fields, functions, lifecycle blocks — in a WebView panel.

Open via: Command Palette: `PrSM: Visualize Script Structure`

### Show Generated C#

Opens the generated `.cs` file side-by-side with the current `.prsm` source. Uses `.prsmmap.json` to jump to the corresponding position in the generated code.

Open via: **Ctrl+Shift+G** or Command Palette: `PrSM: Show Generated C#`

### Show Original PrSM Source

The reverse of Show Generated C#. When viewing a generated `.cs` file, jumps back to the original `.prsm` source at the corresponding position.

Open via: Command Palette: `PrSM: Show Original PrSM Source`

### Insert Lifecycle Block

Quick-pick menu to insert a lifecycle block (`awake`, `start`, `update`, etc.) at the cursor position.

Open via: **Ctrl+Shift+L** or right-click context menu: `PrSM: Insert Lifecycle Block`

### Open from Stack Trace

Parses a Unity/C# stack trace in the editor and navigates to the original `.prsm` source location using source map remapping. Works with Unity Console output pasted into VS Code.

Open via: **Ctrl+Shift+T** or right-click context menu: `PrSM: Open Source from Stack Trace`

### Compile Commands

| Command | Description |
|---------|-------------|
| `PrSM: Compile Current File` | Compiles the active `.prsm` file |
| `PrSM: Compile Workspace` | Compiles all `.prsm` files in the workspace |
| `PrSM: Check Current File` | Runs diagnostics without generating output |

## Keyboard Shortcuts

| Shortcut | Command | Context |
|----------|---------|---------|
| **Ctrl+Shift+G** | Show Generated C# | `.prsm` file active |
| **Ctrl+Shift+V** | Graph View | `.prsm` file active |
| **Ctrl+Shift+L** | Insert Lifecycle Block | `.prsm` file active |
| **Ctrl+Shift+T** | Open from Stack Trace | Any editor |
| F12 | Go to Definition | `.prsm` file active |
| Shift+F12 | Find All References | `.prsm` file active |
| F2 | Rename Symbol | `.prsm` file active |
| Ctrl+Shift+O | Document Symbols | `.prsm` file active |

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `prsm.compilerPath` | `""` (auto-detect) | Path to `prism` binary |
| `prsm.checkOnSave` | `true` | Run diagnostics on file save |
| `prsm.showWarnings` | `true` | Show warning-level diagnostics |
| `prsm.unityApiDbPath` | `""` (bundled) | Path to Unity API SQLite database |

## Trusted Workspace

Compiler-driven features (diagnostics, navigation, rename, symbols, compile commands) only activate in **trusted workspaces**. Syntax highlighting, snippets, and basic editing work everywhere.

To trust a workspace: **File > Manage Workspace Trust**.

## Status Bar

The extension shows its state in the status bar:

| State | Indicator |
|-------|-----------|
| LSP running | `$(check) PrSM (LSP)` |
| LSP starting | `$(sync~spin) PrSM (LSP)` |
| LSP stopped | `$(warning) PrSM (LSP stopped)` |
| Legacy mode | `$(check) PrSM (legacy)` |
| Errors found | `$(error) PrSM: N error(s)` |
| Warnings found | `$(warning) PrSM: N warning(s)` |
