# Contributing

Thank you for contributing to `systemd-tui`.

## Development setup

1. Fork the repository.
2. Clone your fork.
3. Install Rust (stable toolchain).
4. Build and test locally:

```bash
cargo build
cargo test
```

## Development workflow

1. Create a branch from `main`:

```bash
git checkout -b feat/my-change
```

1. Make focused changes (one logical change per PR if possible).
2. Run checks before opening a PR:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

1. Push your branch and open a Pull Request.

## Pull Request guidelines

- Keep PRs small and reviewable.
- Add or update tests for behavior changes.
- Update docs when CLI flags, configuration, or behavior changes.
- Explain the motivation and expected outcome in the PR description.

## Commit guidelines

- Use clear commit messages in imperative mood.
- Prefer one intent per commit (feature, fix, refactor, docs, tests).

## Reporting issues

When reporting a bug, include:

- OS and environment details
- Reproduction steps
- Expected vs actual behavior
- Logs or error messages

