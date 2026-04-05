# M050: Public Docs Truth Reset

## Vision
Make the public Mesh docs truthful to the M049 scaffold/example contract so evaluators and active builders land on one clear first-contact story: install Mesh, run hello-world, choose the clustered scaffold or the explicit SQLite/Postgres Todo starter that matches their goal, and branch into low-level distributed primitives or deeper proof runbooks only when they intentionally want that depth.

## Slice Overview
| ID | Slice | Risk | Depends | Done | After this |
|----|-------|------|---------|------|------------|
| S01 | Onboarding Graph & Retained Rail Reset | High — current sidebar/prev-next behavior and active retained M047/M049 contracts still encode proof-heavy routing, so this slice must change both the public path and the mechanical blockers together. | — | ✅ | On a fresh docs build, the default public path moves through Getting Started and Clustered Example before any proof pages, and updated retained docs contracts fail closed if proof surfaces regain primary-path prominence. |
| S02 | First-Contact Docs Rewrite | Medium-high — these pages currently contain the biggest evaluator-facing drift, and command/link mistakes here directly break the milestone’s primary user loop. | S01 | ✅ | A builder can read Getting Started, run hello-world, then choose `meshc init --clustered`, `meshc init --template todo-api --db sqlite`, or `meshc init --template todo-api --db postgres`, with Tooling reinforcing the same story and exposing the assembled docs-truth command. |
| S03 | Secondary Docs Surfaces & Two-Layer Truth | Medium — the remaining challenge is to keep deeper proof material honest and public without letting it dominate first contact or blur the low-level/runtime-owned split. | S01, S02 | ✅ | From the public docs, a builder can intentionally branch into Distributed Actors, Distributed Proof, or Production Backend Proof, understand each page’s role, and run targeted or assembled verification without those pages regressing into the default onboarding route. |
