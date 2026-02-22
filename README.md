# Natsuzora

Natsuzora（夏空）は、表示専用のミニマルなテンプレート言語です。静的HTML生成やアプリ内テンプレートでの利用を想定しています。

## 特徴

- ロジック最小限で決定的評価（同じ入力なら常に同じ出力）
- デフォルトでHTMLエスケープ（`{[!unsecure ... ]}` のみ非エスケープ）
- 副作用・外部参照なし（DB/HTTP/乱数/現在時刻などを使わない）
- Rust実装とRuby実装を提供

## 現行仕様

- 言語仕様: v4.0
- ファイル拡張子: `.ntzr`
- デリミタ: `{[` ... `]}`

## クイック構文

```ntzr
{[ user.name ]}              <!-- 変数展開（HTMLエスケープあり） -->
{[ user.name? ]}             <!-- nullable modifier -->
{[ user.name! ]}             <!-- required modifier -->

{[#if user.active]}...{[#else]}...{[/if]}
{[#unless has_error]}...{[/unless]}
{[#each items as item]}...{[/each]}

{[!unsecure trusted_html ]}  <!-- 非エスケープ出力 -->
{[!include /components/card title=item.title ]}
{[% this is a comment ]}     <!-- コメント -->

{[{]}                        <!-- リテラル "{[" -->
```

空白制御:

```ntzr
{[-#each items as item-]}
<li>{[ item ]}</li>
{[-/each-]}
```

## ディレクトリ構成

```text
natsuzora/
├── spec/                         # 言語仕様（spec.md, bnf.md）
├── tests/                        # 共有仕様テスト（JSON）
├── rust/
│   ├── Cargo.toml                # workspace
│   └── crates/
│       ├── natsuzora/            # Rust公開API
│       ├── natsuzora-ast/        # AST/parse層
│       ├── natsuzora-ffi/        # C FFI層
│       └── tree-sitter-natsuzora/
├── ruby/                         # Ruby gem
└── tree-sitter/                  # tree-sitter grammar
```

## Rust実装

```bash
cd rust
cargo test -p natsuzora
```

```rust
use serde_json::json;

let html = natsuzora::render(
    "Hello, {[ name ]}!",
    json!({"name": "World"}),
).unwrap();
assert_eq!(html, "Hello, World!");
```

includeあり:

```rust
use serde_json::json;

let html = natsuzora::render_with_includes(
    "{[!include /components/header ]}",
    json!({}),
    "templates/shared",
).unwrap();
```

## Ruby実装

```bash
cd ruby
bundle install
bundle exec rspec
```

```ruby
result = Natsuzora.render(
  "Hello, {[ name ]}!",
  { "name" => "World" }
)
```

### Ruby FFIバックエンド（任意）

Rust実装をRubyから使う場合:

```bash
cd ruby
bundle exec rake build_ffi
NATSUZORA_BACKEND=ffi bundle exec rspec
```

必要に応じて `NATSUZORA_LIB_PATH` で共有ライブラリパスを上書きできます。

## 仕様

詳細は `spec/spec.md` を参照してください。

## ライセンス

MIT
