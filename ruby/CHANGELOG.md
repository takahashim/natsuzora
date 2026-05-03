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
  `Natsuzora::Contract.validate_with_target(contract_file, data, target:)`.
- Shared JSON tests under `tests/contract/` for cross-language parity with
  the Rust `natsuzora-contract` crate.

### Changed
- `subaru` gem is now obsolete; users should switch to
  `gem 'natsuzora', '~> 0.4'` and replace `Subaru::Foo` references with
  `Natsuzora::Contract::Foo`. `Subaru.parse` / `Subaru.validate` map to
  `Natsuzora::Contract.parse` / `Natsuzora::Contract.validate`.

## 0.2.0

Initial release of the natsuzora template language.
