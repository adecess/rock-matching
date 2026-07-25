# Git hooks

Enable the repository-managed hooks once after cloning:

```sh
git config core.hooksPath .githooks
```

The pre-commit hook checks formatting and runs Clippy for the entire Cargo
workspace. A failed check prevents the commit.
