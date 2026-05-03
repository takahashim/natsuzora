# Subaru 仕様書

バージョン: 1.0

## 概要

Subaru（昴）は、データ構造を記述するためのスキーマ言語である。JSONデータのバリデーション、コード生成、ドキュメント生成などに使用できる。TypeScript、Go、GraphQL などに影響を受けた独自記法を採用している。

## 設計方針

1. **シンプル**: 最小限のキーワードと構文
2. **可読性**: 人間が読み書きしやすい記法
3. **JSONとの親和性**: JSONのサブセットを対象とし、相互変換が容易

## データモデル

Subaru が記述するデータ構造は **JSON のサブセット**である。

### 許可される型

| JSON型 | Subaru での扱い |
|--------|-----------------|
| string | 文字列（UTF-8） |
| number（整数） | 符号付き整数（`-(2^53-1)` から `2^53-1`） |
| boolean | 真偽値（`true` / `false`） |
| null | null |
| array | 配列 |
| object | オブジェクト |

### 禁止される値

| 値 | 理由 |
|----|------|
| 浮動小数点数 | テンプレートでの表示形式が不定になるため禁止。表示前に整数または文字列に変換すること |
| NaN, Infinity | JSON標準外であり、禁止 |

### フィールド名の制約

JSONオブジェクトのキー（フィールド名）は、Subaruの識別子規則に従う必要がある:

- 先頭は小文字（`a-z`）
- 2文字目以降は英数字またはアンダースコア（`A-Z`, `a-z`, `0-9`, `_`）

以下のフィールド名は使用できない:

```json
{
  "user-name": "...",      // NG: ハイフン禁止
  "123abc": "...",         // NG: 数字始まり禁止
  "User": "...",           // NG: 大文字始まり禁止（型名と区別できない）
  "@special": "..."        // NG: 特殊文字禁止
}
```

### 設計意図

Subaru は Natsuzora テンプレートのためのデータ構造を記述する目的で設計された。Natsuzoraテンプレートは表示専用であり、計算やデータ加工は行わない。浮動小数点数の表示形式（小数点以下の桁数、丸め方、ロケール依存の区切り文字など）はアプリケーション側で決定するべきであり、スキーマ層で扱うべきではない。そのため、Subaru は整数と文字列のみを数値型として扱い、浮動小数点数を禁止する。数値の表示が必要な場合は、事前に文字列へ変換することを想定している。

## 字句構造

### 空白と改行

- スペース（` `）とタブ（`\t`）はトークン区切りとして機能し、無視される
- 改行（`\n` または `\r\n`）はフィールド区切りとして機能する
- 連続した改行（空行）は無視される

### 識別子

#### フィールド名（field name）
- 先頭は小文字（`a-z`）
- 2文字目以降は英数字またはアンダースコア（`A-Z`, `a-z`, `0-9`, `_`）
- 例: `name`, `userId`, `created_at`

#### 型名（type name）
- 先頭は大文字（`A-Z`）
- 2文字目以降は英数字またはアンダースコア（`A-Z`, `a-z`, `0-9`, `_`）
- 例: `User`, `Comment`, `BlogPost`

### キーワードとコンテキスト

Subaruには予約語がない。`type`, `string`, `integer`, `bool`, `scalar` はすべてフィールド名としても使用できる。

これらの識別子は**文脈によって**解釈が決まる:

| 識別子 | 型位置（`:` の後） | フィールド位置（`:` の前） |
|--------|-------------------|--------------------------|
| `type` | エラー | フィールド名として有効 |
| `string` | 組み込み型 | フィールド名として有効 |
| `integer` | 組み込み型 | フィールド名として有効 |
| `bool` | 組み込み型 | フィールド名として有効 |
| `scalar` | 組み込み型 | フィールド名として有効 |

```
# すべて有効
type: string          # フィールド名 "type"、型 string
string: string        # フィールド名 "string"、型 string
integer: integer      # フィールド名 "integer"、型 integer
```

`type` キーワードは **`type` + 大文字始まりの型名** というパターンでのみ型定義として認識される:

```
type User { }         # 型定義（type + User）
type: string          # フィールド（type + :）
type { name: string } # フィールド（type + {）
```

### コメント

`#` から行末までがコメントとなる。コメントはパース時に無視される。

```
# これはコメント
name: string  # 行末コメントも可能
```

## 基本型

| 型名 | 説明 |
|------|------|
| `string` | 文字列のみ |
| `integer` | 整数のみ |
| `bool` | 真偽値のみ |
| `scalar` | `string \| integer`（文字列化可能な値、`bool`は含まない） |

### 型の使い分け

- **文字列化が必要な値** → `scalar`（string または integer）
- **条件判定に使用する値** → `bool`
- **明確に文字列のみ**の場合 → `string`
- **明確に整数のみ**の場合 → `integer`

## 修飾子

修飾子は基本型の後ろに付与し、null および空文字列の扱いを指定する。
オブジェクト、配列には付与できない。

| 修飾子 | 意味 | バリデーション |
|--------|------|---------------|
| （なし） | null不可、空文字列OK | `null` → エラー、`""` → OK |
| `?` | null許容 | `null` → OK、`""` → OK |
| `!` | null不可、空文字列も不可 | `null` → エラー、`""` → エラー |

### 例

```
name: string      # null不可、空文字列OK
name: string?     # null許容
name: string!     # null不可、空文字列も不可
count: integer    # null不可
count: integer?   # null許容
isActive: bool    # 真偽値
value: scalar     # string | integer、null不可
value: scalar?    # string | integer | null
value: scalar!    # string | integer、null不可、空不可
```

## 型定義

`type` キーワードを使って名前付き型を定義できる。

### 構文

```
type 型名 {
  フィールド定義...
}
```

### 規則

- 型名は大文字始まり（PascalCase）
- 型定義はファイル先頭に配置する
- 循環参照は許容される

### 例

```
type Comment {
  user {
    name: string
  }
  body: string!
}

type Profile {
  name: string
  bio: string?
}

type Author {
  profile: Profile
}
```

## オブジェクト

オブジェクトは `{` `}` で囲んで定義する。

```
user {
  name: string!
  email: string?
  age: integer
}
```

ネストも可能:

```
article {
  title: string!
  author {
    name: string!
    bio: string?
  }
}
```

型参照を使用:

```
article {
  author: Author
  featured: bool
  comments: []Comment
}
```

## 配列

配列は `[]` を型の前に付与する。

### スカラー配列

```
tags: []string        # 文字列の配列
numbers: []integer    # 整数の配列
flags: []bool         # 真偽値の配列
values: []scalar      # string | integer の配列
```

### 型参照配列

```
comments: []Comment   # Comment型の配列
```

### インラインオブジェクト配列

```
items: []{
  title: string!
  count: integer
}
```

## フィールド区切り

フィールドは**改行**で区切る。

```
type User {
  name: string
  age: integer
}
```

## コメント

`#` から行末までがコメントとなる。

```
# ページテンプレート用コントラクト
title: string!       # ページタイトル（必須）
description: string? # 説明（省略可）
```

## 完全な例

```
# 型定義
type Comment {
  user {
    name: string
    avatar: string?
  }
  body: string!
  createdAt: string
}

type Profile {
  name: string!
  bio: string?
  avatar: string?
}

type Author {
  profile: Profile
  isAdmin: bool
}

# ルート定義
title: string!
description: string?

article {
  author: Author
  featured: bool
  comments: []Comment
  tags: []string
}
```

## ファイル構成

```
[型定義セクション]
type TypeA { ... }
type TypeB { ... }

[ルート定義セクション]
field1: type
field2 { ... }
```

型定義はルート定義より前に配置する。

## ファイル拡張子

Subaru ファイルの推奨拡張子は `.sbr` である。

## 2世代記法（Migration Markers）

コントラクトファイルに「現行」と「次期」の2世代分の情報を持たせることで、テンプレートとJSONバックエンドの段階的な移行を支援する。

### 差分マーカー

| マーカー | 意味 | 構文 |
|----------|------|------|
| `+` | 次期で追加 | `+ field: type` |
| `-` | 次期で削除 | `- field: type` |
| `*` | 次期で変更 | `* field: old_type -> new_type` |

### 使用例

```
# 型定義にもマーカー適用可能
+ type NewType {
    name: string
}

- type DeprecatedType {
    old: string
}

type User {
  name: string
  + email: string        # 次期で追加
  - legacyId: integer    # 次期で削除
  * age: integer -> scalar   # 型変更
  * bio: string -> string?   # 修飾子変更
}

# ルート定義
title: string!
+ subtitle: string?
- oldField: scalar

items: []{
  name: string
  + price: integer    # 配列内オブジェクトでも使用可能
}
```

### バリデーションモード

差分マーカー付きコントラクトは、現行世代または次期世代のいずれかでバリデーションできる。

| マーカー | current（現行） | next（次期） |
|----------|-----------------|--------------|
| `+ field` | 無視（存在しない） | 必須 |
| `- field` | 必須 | 無視（存在しない） |
| `* field: A -> B` | 型A | 型B |
| （マーカーなし） | そのまま | そのまま |

### 制約

1. **2世代のみ**: 差分マーカーのネストは禁止
   ```
   + + field: string   # エラー: 2重マーカー
   ```

2. **変更マーカーの構文**: `*` は必ず `->` を含む
   ```
   * field: integer -> scalar   # OK
   * field: integer             # エラー: -> がない
   ```

3. **型定義の変更マーカー**: `type` 自体には `*` は使えない
   ```
   + type NewType { ... }       # OK: 型追加
   - type OldType { ... }       # OK: 型削除
   * type ChangedType { ... }   # エラー: 型の変更は内部フィールドで表現
   ```

詳細な文法制約は `subaru-bnf.md` の「差分マーカーの制約」セクションを参照。

### ワークフロー

1. **差分検出**: テンプレートから抽出したコントラクトと既存コントラクトを比較し、差分マーカーを自動付与
2. **移行期間**: `current` と `next` の両方でバリデーション可能
3. **確定**: 移行完了後、差分マーカーを消して次期を現行に適用

## BNF

詳細な文法定義は `subaru-bnf.md` を参照。
