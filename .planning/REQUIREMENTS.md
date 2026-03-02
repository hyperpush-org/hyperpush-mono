# Requirements: Mesh

**Defined:** 2026-03-01
**Milestone:** v15.0 Package Dogfood
**Core Value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.

## v15 Requirements

Build, publish, and consume a real Mesh package (mesh-slug) to validate the end-to-end package manager workflow.

### Slug Library

- [ ] **SLUG-01**: User can call `Slug.slugify(str)` to convert any string to a URL-safe slug (lowercase, non-alphanumeric chars replaced with separator, consecutive separators collapsed, leading/trailing separators stripped)
- [ ] **SLUG-02**: User can call `Slug.slugify(str, sep)` with a custom separator string
- [ ] **SLUG-03**: User can call `Slug.truncate(slug, max)` to truncate a slug to at most max characters, cutting at the last separator boundary
- [ ] **SLUG-04**: User can call `Slug.is_valid(slug)` to check if a string is already a valid slug (returns Bool)
- [ ] **SLUG-05**: mesh-slug has unit tests via `meshc test` covering normal cases and edge cases (empty string, all-special-char input, long strings)

### Distribution

- [ ] **DIST-01**: mesh-slug has a valid `mesh.toml` manifest with name, version, and description fields
- [ ] **DIST-02**: User can publish mesh-slug to the package registry via `meshpkg publish`
- [ ] **DIST-03**: mesh-slug appears in search results on packages.meshlang.dev after publishing
- [ ] **DIST-04**: User can install mesh-slug in a project via `meshpkg install mesh-slug`

### Mesher Integration

- [ ] **INTG-01**: Mesher's `mesh.toml` declares `mesh-slug` as a dependency with its published version
- [ ] **INTG-02**: `meshpkg install` successfully downloads and installs mesh-slug into Mesher
- [ ] **INTG-03**: Mesher imports mesh-slug and uses `Slug.slugify` for project slug generation
- [ ] **INTG-04**: Mesher compiles and all existing functionality works correctly with the mesh-slug dependency

## Future Requirements

None identified for now.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Unicode transliteration (café → cafe) | Complex, not needed for v15 scope |
| Multiple published packages | One package is sufficient to validate the workflow |
| meshpkg install from local path | Registry flow is the target; local install is already tested via dev workflow |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SLUG-01 | — | Pending |
| SLUG-02 | — | Pending |
| SLUG-03 | — | Pending |
| SLUG-04 | — | Pending |
| SLUG-05 | — | Pending |
| DIST-01 | — | Pending |
| DIST-02 | — | Pending |
| DIST-03 | — | Pending |
| DIST-04 | — | Pending |
| INTG-01 | — | Pending |
| INTG-02 | — | Pending |
| INTG-03 | — | Pending |
| INTG-04 | — | Pending |

**Coverage:**
- v15 requirements: 13 total
- Mapped to phases: 0
- Unmapped: 13 ⚠️

---
*Requirements defined: 2026-03-01*
*Last updated: 2026-03-01 after initial definition*
