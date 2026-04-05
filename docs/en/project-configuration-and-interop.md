---
title: Project Configuration & Interop
parent: Advanced
grand_parent: English Docs
nav_order: 1
---

# Project Configuration & Interop

Project-level configuration is driven by `.prsmproject`.

Current configuration areas include:

- project identity
- language version and feature flags
- compiler path and output directory
- source include/exclude patterns

Interop is centered on generated C# that stays readable and Unity-friendly.

- components lower to normal Unity component classes
- coroutines lower to `IEnumerator`
- assets lower to ScriptableObject-backed classes
- enum payloads lower through generated extension methods

The compiler still contains some compatibility fallbacks for older naming, but `.prsmproject` is the active project format.
