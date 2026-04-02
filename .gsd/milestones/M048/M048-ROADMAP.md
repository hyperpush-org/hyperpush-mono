# M048: 

## Vision
Mesh keeps `main.mpl` as the simple default but stops hardcoding it everywhere, and the first-contact tooling surfaces — self-update, editor grammar, and init-time skills — match the current language/runtime contract instead of stale repo assumptions.

## Slice Overview
| ID | Slice | Risk | Depends | Done | After this |
|----|-------|------|---------|------|------------|
| S01 | Configurable entrypoint in compiler and test discovery | high | — | ✅ | After this: a real Mesh project can keep root `main.mpl` or use an overridden entry file such as `lib/start.mpl`, and compiler build plus test discovery both honor the same executable contract. |
| S02 | Entrypoint-aware LSP, editors, and package surfaces | high | S01 | ✅ | After this: the same non-`main.mpl` project opens cleanly in LSP/editor flows and package/discovery surfaces stop treating root `main.mpl` as the only valid executable contract. |
| S03 | Toolchain self-update commands | medium | S01 | ✅ | After this: installed or staged `meshc` and `meshpkg` expose explicit self-update commands that refresh the toolchain through the same release/install path users already trust. |
| S04 | Syntax and init-skill parity reset | medium | S02 | ✅ | After this: VS Code and Vim highlight `@cluster` and both interpolation forms correctly, and the Mesh init-time skill bundle teaches the current clustered/runtime story instead of stale pre-reset guidance. |
| S05 | Assembled contract proof and minimal public touchpoints | medium | S02, S03, S04 | ⬜ | After this: one retained verifier proves the override-entry project, self-update commands, grammar parity, and refreshed skill contract together, and the minimal public touchpoints stop lying about these surfaces. |
