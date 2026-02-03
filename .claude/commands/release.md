---
name: release
description: Prepare a new release - bump version, update CHANGELOG from git log, show tag commands
---

# Release Preparation

## Process

1. **Get current version** from `Cargo.toml`
2. **Calculate next version** (default: patch increment)
3. **Ask user** to confirm version (patch/minor/major or custom)
4. **Generate changelog** from git commits since last tag
5. **Update files:**
   - `CHANGELOG.md` - Add new version section with categorized changes
   - `Cargo.toml` - Bump version
6. **Show release commands** (do NOT execute):
   ```
   git add -A && git commit -m "chore: release vX.Y.Z"
   git tag vX.Y.Z
   git push origin main --tags
   ```

## Changelog Generation

Categorize commits by conventional commit prefix:

| Prefix | Category |
|--------|----------|
| `feat:` | Added |
| `fix:` | Fixed |
| `docs:` | Documentation |
| `refactor:` | Changed |
| `perf:` | Performance |
| `test:` | Tests |
| `chore:`, `ci:` | Skip (internal) |

Format each entry as: `- {commit message without prefix}`

## CHANGELOG.md Format

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- Feature descriptions

### Fixed
- Bug fix descriptions

[X.Y.Z]: https://github.com/tomoya55/jlcat/releases/tag/vX.Y.Z
```

## Important

- Do NOT auto-commit or auto-tag
- Do NOT push anything
- Show commands for user to execute manually
- Always ask for version confirmation before making changes
