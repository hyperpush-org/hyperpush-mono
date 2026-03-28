# M038: 

## Vision
Fix the Windows `meshc.exe build` access violation in the hosted release workflow so all six hosted workflow lanes are green on the release tag.

## Slice Overview
| ID | Slice | Risk | Depends | Done | After this |
|----|-------|------|---------|------|------------|
| S01 | Fix Windows MSVC Build and Verify Release Lane | high | — | ⬜ | The hosted `release.yml` Windows smoke job goes green. `meshc.exe build` on the trivial fixture produces a working executable without `/FORCE:MULTIPLE`. |
