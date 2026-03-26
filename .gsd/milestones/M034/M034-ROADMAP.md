# M034: 

## Vision
Harden CI/CD and ensure everything important is included in it, test the package manager end to end, and turn Mesh’s public release path into something that is proven rather than assumed.

## Slice Overview
| ID | Slice | Risk | Depends | Done | After this |
|----|-------|------|---------|------|------------|
| S01 | Real registry publish/install proof | high | — | ✅ | A release-scoped package can be published to the real registry path and installed with `meshpkg`, with checksum, metadata, download, and lockfile truth rechecked. |
| S02 | Authoritative CI verification lane | high | S01 | ⬜ | PR and release verification rerun the real Mesh proof surfaces, including the package-manager path, instead of stopping at artifact builds. |
| S03 | Release assets and installer truth | medium | S01, S02 | ⬜ | Released `meshc` and `meshpkg` artifacts are proven installable and runnable through the documented installer path instead of only being uploaded. |
| S04 | Extension release path hardening | medium | S02 | ⬜ | The VS Code extension publish lane validates the packaged extension and release prerequisites before public publication. |
| S05 | Full public release assembly proof | high | S01, S02, S03, S04 | ⬜ | One release candidate is proven across binaries, installer, docs deployment, registry/packages-site health, and extension release checks as a single public-ready flow. |
