---
name: release
description: Prepare a new release - bump version, update CHANGELOG, commit, create PR, show tag commands
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
6. **Commit and push:**
   - `git add CHANGELOG.md Cargo.toml && git commit -m "chore: release vX.Y.Z"`
   - `git push -u origin <branch>`
7. **Create PR** using `gh pr create`
8. **Show release commands** with alert:
   ```
   ⚠️  Run these commands AFTER the PR is merged:

   git checkout main && git pull
   git tag vX.Y.Z
   git push origin --tags
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

- Do NOT auto-tag (user runs tag commands after PR merge)
- Always ask for version confirmation before making changes
- Commit, push, and create PR automatically
- Show tag commands with clear alert about running after merge
