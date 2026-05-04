# Changelog

## 0.4.0

### Added
- `Natsuzora::Contract` namespace which absorbs the formerly separate `subaru` gem.
  Provides parsing of `.ntzc` contract notation and validation of JSON data
  against contracts. Two-generation diff markers (`+`, `-`, `*`) are supported.
- Module-level helpers: `Natsuzora::Contract.parse(input)`,
  `Natsuzora::Contract.parse_file(input)`,
  `Natsuzora::Contract.parse_file_with_diff(input)`,
  `Natsuzora::Contract.validate(contract, data)`,
  `Natsuzora::Contract.validate_with_target(document, data, target:)`.
- Contract AST nodes are grouped under `Natsuzora::Contract::AST::`
  (`Any`, `Scalar`, `Record`, `List`, `Ref`, with `Node` as the abstract
  base). `Natsuzora::Contract::AST.from_h(hash)` rebuilds an AST tree
  from its hash representation.
- `Natsuzora::Contract::TypeRefResolver` walks an AST tree replacing
  `AST::Ref` nodes with concrete contracts; configurable via
  `on_missing` / `on_unavailable` callbacks.
- Shared JSON tests under `tests/contract/` for cross-language parity with
  the Rust `natsuzora-contract` crate.
- `Natsuzora::Payload` class wrapping render input data. The class
  encapsulates the boundary between untrusted host data and Natsuzora's
  internal value space (Symbol→String key adaptation, whole-number
  Float→Integer coercion, plus type/range validation).
- `Natsuzora::DataNormalizable` mixin providing `normalize_data(data)` for
  pure data adaptation (no exceptions raised).
- `Natsuzora::Validator.validate_data!(data)` for asserting that a
  prepared value conforms to the Natsuzora type system.

### Changed
- **Breaking** (pre-1.0): `Natsuzora::Template#render` now accepts a
  `Natsuzora::Payload` instead of a raw `Hash`. The top-level facade
  `Natsuzora.render(source, data, ...)` is unchanged and wraps `data` in a
  Payload internally; only callers that construct a `Template` directly
  via `Natsuzora.parse` need to update.

  ```ruby
  # Before
  template = Natsuzora.parse(source)
  template.render(hash)

  # After
  template = Natsuzora.parse(source)
  template.render(Natsuzora::Payload.new(hash))
  ```

## 0.2.0

Initial release of the natsuzora template language.
