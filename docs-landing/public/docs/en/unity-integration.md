---
title: Unity Integration
parent: Tooling
grand_parent: English Docs
nav_order: 2
---

# Unity Integration

The Unity package already implements a practical import/build workflow for `.prsm` files.

Current responsibilities include:

- importing and compiling PrSM source inside Unity workflows
- generated C# output handling
- compiler resolution helpers
- project settings integration
- source-map aware diagnostic remapping
- templates and editor menu actions

This is not just future design work. The repository already includes package code and smoke verification around these flows.
