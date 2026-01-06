# Natsuzora

Natsuzora（夏空）は、表示専用のミニマルなテンプレート言語です。静的HTML生成とRailsプレビューテンプレートに使用されます。

## 特徴

- ロジック最小限、すべての評価は決定的
- デフォルトですべての出力をHTMLエスケープ
- 副作用・外部参照なし（DB、HTTP、乱数、現在時刻などは禁止）
- Rust と Ruby の両実装を提供

## ディレクトリ構成

```
natsuzora/
├── spec/           # 言語仕様
│   ├── spec.md     # 完全な言語仕様 v1.8
│   └── bnf.md      # BNF/EBNF形式の構文定義
├── tests/          # 共有テストケース
│   └── *.json      # 両実装で使用するテストデータ
├── rust/           # Rust実装
│   ├── src/
│   └── Cargo.toml
└── ruby/           # Ruby実装
    ├── lib/
    └── natsuzora.gemspec
```

## 基本構文

### 変数展開

```
{{ path.to.value }}
```

### 制御構造

```
{{#if expr}}...{{#else}}...{{/if}}
{{#unless expr}}...{{/unless}}
{{#each items as item}}...{{/each}}
{{#each items as item, index}}...{{/each}}
{{#unsecure}}...{{/unsecure}}
```

### コメント

```
{{! このコメントは出力に含まれない }}
```

### 空白制御

```
{{-#each items as item-}}
<li>{{ item }}</li>
{{-/each-}}
```

### パーシャル（include）

```
{{> /components/card}}
{{> /components/card key=value}}
```

## Rust実装

```bash
cd rust
cargo build
cargo test
```

```rust
use serde_json::json;

let result = natsuzora::render(
    "Hello, {{ name }}!",
    json!({"name": "World"}),
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
  "Hello, {{ name }}!",
  { "name" => "World" }
)
```

## 仕様

詳細は [spec/spec.md](spec/spec.md) を参照してください。

## ライセンス

MIT License
