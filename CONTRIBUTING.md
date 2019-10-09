# Contributor Guidelines

Contributors MUST follow these rules:

- Discuss a feature before implementing it
- Run `cargo +nightly fmt` on your code before committing it
- Commits should follow the [Conventional Commit] guidelines
    - A small change in one commit is exempt from this rule

Things that contributors SHOULD be aware of:

- `rustup` is the preferred tool for managing Rust toolchains
- `cargo clippy` prevents the majority of common mistakes
- `unsafe` is explicitly disallowed, unless otherwise permitted by a maintainer
- `sbuild` verifies that debian packages build correctly

[conventional commit]: https://www.conventionalcommits.org/en/v1.0.0-beta.4/
