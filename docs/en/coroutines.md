---
title: Coroutines
parent: Language Guide
grand_parent: English Docs
nav_order: 10
---

# Coroutines

Coroutines are a first-class feature in PrSM. They lower to Unity's `IEnumerator`-based coroutine system and are started, stopped, and waited on through dedicated syntax.

## Declaring a coroutine

Use the `coroutine` keyword:

```prsm
coroutine spawnWave() {
    for i in 0 until waveSize {
        spawn(enemyPrefab)
        wait 0.5s
    }
}
```

## Wait forms

PrSM provides several `wait` forms that map to Unity's yield instructions:

| Syntax | Unity equivalent |
|---|---|
| `wait 1.0s` | `yield return new WaitForSeconds(1.0f)` |
| `wait nextFrame` | `yield return null` |
| `wait fixedFrame` | `yield return new WaitForFixedUpdate()` |
| `wait until condition` | `yield return new WaitUntil(() => condition)` |
| `wait while condition` | `yield return new WaitWhile(() => condition)` |

```prsm
coroutine hitInvincible() {
    invincible = true
    wait invincibleTime.s     // invincibleTime is Float
    invincible = false
}

coroutine waitForDoor() {
    wait until door.isOpen
    enterRoom()
}
```

## Control forms

| Syntax | Effect |
|---|---|
| `start coroutineName()` | Starts the coroutine (calls `StartCoroutine`) |
| `stop coroutineName()` | Stops the named coroutine |
| `stopAll()` | Stops all running coroutines on this component |

```prsm
awake {
    start spawnWave()
}

onDestroy {
    stopAll()
}
```

## Duration literals

A number followed by `.s` is a duration literal and is emitted as a float representing seconds:

```prsm
wait 2.5s
wait cooldown.s
```
