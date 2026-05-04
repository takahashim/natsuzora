# natsuzora

A minimal, display-only template language for safe HTML generation.

## Features

- HTML-escaped by default; only `{[!unsecure ... ]}` outputs raw HTML.
- Deterministic — no side effects, no I/O, no globals.
- `include` resolution rooted at a configured directory; symlinks and
  `..` paths are rejected.
- Bounded recursion and output size guard against pathological input.

## Example

```rust
use serde_json::json;

let html = natsuzora::render(
    "Hello, {[ name ]}!",
    json!({"name": "World"}),
)?;
assert_eq!(html, "Hello, World!");
# Ok::<_, natsuzora::NatsuzoraError>(())
```

`{[!unsecure ... ]}` must only be used for trusted HTML fragments:

```rust
use serde_json::json;

let html = natsuzora::render(
    "{[!unsecure trusted_html ]}",
    json!({"trusted_html": "<strong>OK</strong>"}),
)?;
# Ok::<_, natsuzora::NatsuzoraError>(())
```

With includes:

```rust
use serde_json::json;

let html = natsuzora::render_with_includes(
    "{[!include /components/header ]}",
    json!({}),
    "templates/shared",
)?;
# Ok::<_, natsuzora::NatsuzoraError>(())
```

## Syntax overview

```ntzr
{[ user.name ]}              variable, HTML-escaped
{[ user.name? ]}             nullable modifier
{[ user.name! ]}             required modifier
{[#if cond]}...{[#else]}...{[/if]}
{[#unless cond]}...{[/unless]}
{[#each items as item]}...{[/each]}
{[!unsecure trusted_html ]}  raw output
{[!include /partial key=val ]}
{[% comment ]}
```

The full language specification lives in
[`spec/spec.md`](https://github.com/takahashim/natsuzora/blob/main/spec/spec.md).

## License

MIT
