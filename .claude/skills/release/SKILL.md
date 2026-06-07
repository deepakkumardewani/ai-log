---
name: release
description: Drive a weavr release end-to-end — version bump, CI gate, tag, GitHub Release, crates.io publish, and Homebrew tap update — pausing for explicit approval at the two irreversible gates. Use when the user wants to "release", "cut a release", "ship a new version", "publish to crates.io", "bump the version", or "tag a release" for the weavr crate.
---

# Release weavr

Drive a weavr release from a clean `main` to all three distribution channels —
GitHub Releases, crates.io, and the Homebrew tap — running the rote steps yourself
and **stopping for an explicit go/no-go before each one-way door**.

This skill encodes a process that is otherwise a string of easy-to-forget manual
steps performed months apart. Follow it top to bottom; do not skip the gates.

## The drive-but-ask contract

- **You run** the rote steps: version bump, `just ci`, dry-run, commit, watching
  the Action, the brew-formula script, install verification.
- **You stop and ask** for an explicit "yes" before the two irreversible actions:
  1. **Pushing the `vX.Y.Z` tag** (it triggers the public Release workflow and can't be cleanly un-pushed).
  2. **`cargo publish`** (a crates.io version can never be republished or truly deleted — only yanked).

Never perform either gated action without the user typing an explicit yes for that
specific step. "Looks good", silence, or "whatever you think" are **not** a yes —
re-ask with the concrete command you are about to run.

## Branching: tag on `main` only

weavr is solo and releases from a single moving `main`. The rule this skill enforces:

> **Releases happen only from a clean, up-to-date `main`.** Never tag a feature
> branch or a dirty tree.

You do **not** use release or maintenance branches today. Do not create them.

<details>
<summary>When you outgrow tag-on-main → maintenance branches</summary>

Add a maintenance-branch flow only when **both** become true: you've moved `main`
on to a new major (e.g. 2.0) **and** a user needs a patch to an old major they
can't upgrade off of. At that point: create `release/1.x` *from the `v1.x.y` tag*
(not from `main`), cherry-pick the fix onto it, tag `v1.x.(y+1)` on that branch.
The branch is long-lived and never merges back to `main`. Until that day, this
section is documentation, not a step.
</details>

## Hard preconditions — refuse to proceed if any fail

Check all of these first. If any fails, stop and report it; do not work around it.

1. **On `main`:** `git rev-parse --abbrev-ref HEAD` is `main`.
2. **Clean tree:** `git status --porcelain` is empty.
3. **Synced with remote:** `git fetch` then confirm `main` is not behind/ahead of `origin/main`.
4. **CI is green on the current commit** (the `ci.yml` workflow, separate from release).
5. **The Homebrew tap is checked out next door:** `../homebrew-weavr/Formula/weavr.rb` exists
   (the brew script reads `../../homebrew-weavr` relative to `scripts/`).
6. **crates.io auth is available** for `cargo publish` (token configured, or you'll prompt at publish time).

## The release sequence

### 1. Choose the version (semver)
Ask the user for the target version if not given. Current version is in `Cargo.toml`
(`version = "X.Y.Z"`). Follow semver against the last release: breaking → major,
new feature → minor, fix only → patch. Confirm the chosen `vX.Y.Z` back to the user.

### 2. Bump the version
- Edit `version` in `Cargo.toml`.
- Run `cargo build` (or any cargo command) so `Cargo.lock` updates to the new version.

### 3. Update `CHANGELOG.md`
Add a new section at the top in the existing format:

```
## [X.Y.Z] — YYYY-MM-DD

### Added / Changed / Fixed
- ...
```

Draft entries from the commits since the last tag (`git log vLAST..HEAD --oneline`),
then ask the user to confirm/edit the changelog before continuing.

### 4. CI gate — `just ci`
Run `just ci` (fmt → clippy → test). **Must pass.** If anything fails, stop and fix
before going further — never release red.

### 5. Validate the publish — `cargo publish --dry-run`
Run `cargo publish --dry-run` and confirm it packages cleanly (no uncommitted-file
warnings, no missing metadata). This is a safe rehearsal of step 8.

### 6. Commit & push the bump
- `git add Cargo.toml Cargo.lock CHANGELOG.md`
- `git commit -m "release: prepare vX.Y.Z"`
- `git push origin main`

### 7. 🚪 GATE 1 — tag & push (irreversible)
**Stop. Show the user the exact commands and get an explicit yes.**
```
git tag vX.Y.Z
git push origin vX.Y.Z
```
Pushing the tag triggers `.github/workflows/release.yml`, which builds
`aarch64-apple-darwin` + `x86_64-unknown-linux-gnu`, then creates the public
GitHub Release with archives, `.sha256` checksums, and install instructions.

### 8. Watch the Release workflow & verify
- Watch the run: `gh run watch` (or `gh run list --workflow=release.yml`).
- When green, verify the GitHub Release exists with **both** target tarballs and
  their `.sha256` files: `gh release view vX.Y.Z`.
- If the workflow fails, **do not publish to crates.io** — fix and re-tag first.

### 9. 🚪 GATE 2 — `cargo publish` (irreversible)
**Stop. Get an explicit yes**, then:
```
cargo publish
```
A published crates.io version is permanent (yank-only). Done only after the
GitHub build matrix succeeded in step 8.

### 10. Update the Homebrew tap
The brew formula's URLs are version-interpolated; the script only refreshes the
version and per-target sha256 (it downloads the just-published tarballs, so it
must run after step 8):
```
VERSION=vX.Y.Z ./scripts/update-brew-formula.sh
```
Then commit & push the tap as the script's output instructs:
```
cd ../homebrew-weavr
git add Formula/weavr.rb
git commit -m "weavr vX.Y.Z"
git push origin main
```

### 11. Verify all install paths land the new version
- **crates.io:** version visible at `https://crates.io/crates/weavr` (or `cargo info weavr`).
- **GitHub Release:** `gh release view vX.Y.Z` shows both tarballs + checksums.
- **Homebrew:** `brew update && brew info weavr` (or a clean `brew install deepakkumardewani/weavr/weavr`) reports `vX.Y.Z`.
- **cargo install:** optionally `cargo install weavr --force` resolves to the new version.

## Verification checklist

- [ ] Released from a clean, synced `main` — no feature branch, no dirty tree
- [ ] `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` all reflect `vX.Y.Z`
- [ ] `just ci` passed and `cargo publish --dry-run` was clean before tagging
- [ ] Got an explicit user yes before **both** gates (tag push, `cargo publish`)
- [ ] GitHub Release has both target tarballs + `.sha256` files
- [ ] crates.io shows the new version
- [ ] Homebrew tap committed & pushed with updated version + sha256
- [ ] All install paths verified on the new version
