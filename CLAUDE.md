## Important CLI Tools (use these, not the defaults)

- `cargo` for all Rust build, test, and dependency operations
- `cargo build` for debug builds, `cargo build --release` for release
- `cargo fmt --all` for code formatting
- `cargo clippy --all-targets -- -D warnings` for linting
- `cargo test` for running all tests
- `just ci` for the full CI pipeline (fmt → clippy → test)
- `gh` is installed — use for all Git operations
- **IMPORTANT**: This is a pure Rust project. There is no `package.json`, `node_modules`, or `bun` toolchain. All operations go through `cargo`.

### Rust-Specific Rules

- This is a RUST based project and NOT a FRONTEND project. Whenever using the /build command and related skills, only use the best practices that applies to any software project and ignore all the frontend related project when writing/building/testing rust code.
- Prefer named structs over long parameter lists — avoid "options object" patterns.
- Lean on the type system; make illegal states unrepresentable.
- Use `Result` and `Option` idiomatically; never unwrap in production paths without justification.
- Prefer `thiserror` for library errors, `anyhow` for application errors.
- Keep `unsafe` blocks minimal, isolated, and always documented with a `// SAFETY:` comment.

### Skills Reference

| Task                | Skill                                         |
| ------------------- | --------------------------------------------- |
| Rust Best Practices | `.claude/skills/rust-best-practices/SKILL.md` |
| Rust Testing        | `.claude/skills/rust-testing/SKILL.md`        |
