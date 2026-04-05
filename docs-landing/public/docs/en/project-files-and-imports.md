---
title: Project Files & Imports
parent: Language Guide
grand_parent: English Docs
nav_order: 12
---

# Project Files & Imports

PrSM does not currently ship a separate module system in the Zephyr sense. The implemented scope-management surface is:

- namespace imports through `using`
- project discovery through `.prsmproject`
- source include/exclude globs
- compiler output directory configuration

Example imports:

```prsm
using UnityEngine
using UnityEngine.UI
using UnityEngine.SceneManagement
```

Minimal project file:

```toml
[project]
name = "PrSMDemo"
prsm_version = "0.1.0"

[compiler]
output_dir = "build-output"
```
