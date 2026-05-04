# natsuzora

A minimal, display-only template language for safe HTML generation.

## Features

- HTML-escaped by default; only `{[!unsecure ... ]}` outputs raw HTML.
- Deterministic — templates cannot access globals or perform arbitrary I/O.
- `include` resolution rooted at a configured directory; symlinks and
  `..` paths are rejected.
- Circular includes are rejected.

## Example

```rust
use serde_json::json;

let html = natsuzora::render(
    "Hello, {[ name ]}!",
    json!({"name": "World"}),
).unwrap();
assert_eq!(html, "Hello, World!");
```

`{[!unsecure ... ]}` must only be used for trusted HTML fragments:

```rust
use serde_json::json;

let html = natsuzora::render(
    "{[!unsecure trusted_html ]}",
    json!({"trusted_html": "<strong>OK</strong>"}),
).unwrap();
assert_eq!(html, "<strong>OK</strong>");
```

With includes:

```rust,ignore
use serde_json::json;

let html = natsuzora::render_with_includes(
    "{[!include /components/header ]}",
    json!({}),
    "templates/shared",
).unwrap();
```

`/components/header` resolves under the include root as
`components/_header.ntzr`.

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
