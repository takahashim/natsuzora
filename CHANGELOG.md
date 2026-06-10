# Changelog

Monorepo-wide changelog (spec, Rust, Ruby, tree-sitter).
Gem-specific details: [ruby/CHANGELOG.md](ruby/CHANGELOG.md).

## 0.4.1

### Added
- Differential testing harness between the Rust and Ruby implementations
  (`spec/difftest.md`): `natsuzora-difftest` crate (proptest generator)
  and `ruby/exe/natsuzora-difftest-worker` (JSONL worker).

### Changed
- Comment content is raw-scanned to the first `]}`; any characters are
  now allowed inside `{[% ... ]}` (all implementations).
- Ruby: `LexerError` is now a subclass of `ParseError`.

### Fixed
- Rust: syntax errors inside partials stay `ParseError` (were
  reclassified as `IncludeError`).
- tree-sitter: comment content ending with `]` now parses.

## 0.4.0

- Spec v4.0: variable modifiers `?` / `!`.
- Subaru contract language integrated (`Natsuzora::Contract`,
  `natsuzora-contract` crate, `tests/contract/`).
- Resource limits against pathological input.
- Ruby: render input wrapped in `Natsuzora::Payload` (breaking for
  direct `Template#render` callers).
- Rust: `natsuzora-ast` merged into `natsuzora`; MSRV 1.74.

## 0.2.0

- Initial release: Rust and Ruby implementations, shared spec and test
  suite, tree-sitter grammar.
