# natsuzora-contract

Parser and validator for **natsuzora contract notation** (`.ntzc`) — a
minimal schema language for describing the JSON shape that a
[`natsuzora`](https://crates.io/crates/natsuzora) template expects.

## Features

- Parse `.ntzc` files into an AST (`parse`, `parse_file`).
- Validate JSON data against a contract (`validate`).
- Two-generation diff markers (`+`, `-`, `*`) for staged schema changes
  (`parse_file_with_diff`, `validate_with_target`).
- Static template-vs-contract checking (`check_template`) — verify a
  natsuzora template only references paths declared in the contract.
- CLI tool `natsuzora-contract` for project-wide lint, sync, and apply
  (enabled by default; opt out with `default-features = false`).

## Example

```rust
use natsuzora_contract::{parse, validate};
use serde_json::json;

let contract = parse("name: string!\nage: integer?")?;
validate(&contract, &json!({"name": "Alice", "age": 30}))?;
# Ok::<_, Box<dyn std::error::Error>>(())
```

## Notation

```ntzc
# scalar fields with optional modifiers
name: string!     # required
age:  integer?    # nullable

# nested object
user {
  name: string
}

# array
tags: []string

# named type definition + reference
type Author { name: string }
authors: []Author
```

The complete contract specification lives in
[`spec/contract.md`](https://github.com/takahashim/natsuzora/blob/main/spec/contract.md).

## License

MIT
