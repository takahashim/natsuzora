# Subaru BNF

## 文法定義

```bnf
subaru            ::= type_section root_section
type_section      ::= (type_def | comment | separator)*
root_section      ::= (field | comment | separator)*
type_def          ::= add_remove_marker? "type" type_name "{" field_list "}"
field_list        ::= (field | comment | separator)*
field             ::= diff_marker? field_name (block | ":" type_expr)
block             ::= "{" field_list "}"
type_expr         ::= type_change | type
type_change       ::= type "->" type
type              ::= array_type | scalar_type | type_name
array_type        ::= "[]" (block | scalar_type | type_name)
scalar_type       ::= builtin_type modifier?
builtin_type      ::= "string" | "integer" | "bool" | "scalar"
modifier          ::= "?" | "!"
diff_marker       ::= "+" | "-" | "*"
add_remove_marker ::= "+" | "-"
type_name         ::= upper_letter (letter | digit | "_")*
field_name        ::= lower_letter (letter | digit | "_")*
separator         ::= newline
comment           ::= "#" (any character except newline)* newline?
newline           ::= LF | CR LF
upper_letter      ::= "A"-"Z"
lower_letter      ::= "a"-"z"
letter            ::= upper_letter | lower_letter
digit             ::= "0"-"9"
```

## 字句規則

### 空白

- スペース（` `）とタブ（`\t`）は無視される（トークン区切りとして機能）
- 改行（`\n` または `\r\n`）はフィールド区切りとして機能

### フィールド区切り

- **改行**がフィールド区切りとして機能
- 連続した改行（空行）は無視

### コメント

- `#` から行末までがコメント
- コメントは無視される

### 識別子

#### フィールド名（field name）
- 先頭は小文字（`a-z`）
- 2文字目以降は英数字またはアンダースコア（`A-Z`, `a-z`, `0-9`, `_`）

#### 型名（type name）
- 先頭は大文字（`A-Z`）
- 2文字目以降は英数字またはアンダースコア（`A-Z`, `a-z`, `0-9`, `_`）

### コンテキストキーワード

Subaruには予約語がない。以下の識別子は文脈によって解釈が決まる:

- `type` - 大文字始まりの型名が続く場合のみ型定義キーワード
- `string`, `integer`, `bool`, `scalar` - 型位置では組み込み型、フィールド位置ではフィールド名

#### 型定義の判別

```
type User { }         # type + TypeName → 型定義
type: string          # type + : → フィールド "type"
type { }              # type + { → フィールド "type" (インラインオブジェクト)
```

#### 組み込み型名のフィールド使用

```
string: string        # フィールド名 "string"、型 string
integer: integer      # フィールド名 "integer"、型 integer
```

## 構文例

### 型定義

```
type Comment {
  body: string!
}
```

```
"type" type_name "{" field_list "}"
    ↓
"type" "Comment" "{" ("body" ":" "string" "!") "}"
```

### 単純なフィールド

```
name: string
```

```
field_name ":" scalar_type
    ↓
"name" ":" "string"
```

### 修飾子付きフィールド

```
email: string?
```

```
field_name ":" scalar_type modifier
    ↓
"email" ":" "string" "?"
```

### オブジェクトフィールド

```
user {
  name: string!
  age: integer
}
```

```
field_name block
    ↓
"user" "{" field* "}"
    ↓
"user" "{"
  ("name" ":" "string" "!")
  ("age" ":" "integer")
"}"
```

### 型参照

```
author: Author
```

```
field_name ":" type_name
    ↓
"author" ":" "Author"
```

### 配列（スカラー）

```
tags: []string
```

```
field_name ":" array_type
    ↓
"tags" ":" "[]" "string"
```

### 配列（型参照）

```
comments: []Comment
```

```
field_name ":" array_type
    ↓
"comments" ":" "[]" type_name
    ↓
"comments" ":" "[]" "Comment"
```

### 配列（インラインオブジェクト）

```
items: []{
  title: string!
}
```

```
field_name ":" array_type
    ↓
"items" ":" "[]" block
    ↓
"items" ":" "[]" "{" field* "}"
```

## パース優先順位

1. 型定義セクションのパース（`type` キーワードで始まる定義）
2. ルート定義セクションのパース（残りのフィールド）
3. 各フィールド内:
   - 識別子の読み取り
   - `:` または `{` の判定
   - 型または子ブロックのパース
   - 修飾子の読み取り（あれば）

## エラー処理

以下の場合はパースエラーとなる:

1. 識別子が期待される位置に識別子以外がある
2. `:` または `{` が期待される位置に別のトークンがある
3. `}` が閉じられていない
4. `[]` の後に型名もブロックもない
5. 不明な型名が使用されている（定義されていない型参照）
6. 予期しない文字がある
7. 型名がフィールド名の位置に使用されている（大文字始まり）
8. フィールド名が型名の位置に使用されている（小文字始まりの型参照）

**注意**: `type` はコンテキストキーワードであり、`type: string` や `type { }` のようにフィールド名として使用できる。`type User { }` のパターンのみが型定義として認識される。

## 型の解決

1. 組み込み型（`string`, `integer`, `bool`, `scalar`）は即座に解決
2. ユーザー定義型は型定義セクションから検索
3. 循環参照は許容（遅延解決）
4. 未定義の型参照はエラー

## 差分マーカー（Migration Markers）

### 差分マーカー付きフィールド

```
+ email: string
```

```
diff_marker field_name ":" type
    ↓
"+" "email" ":" "string"
```

### 差分マーカー付き型変更

```
* age: integer -> scalar
```

```
diff_marker field_name ":" type_change
    ↓
"*" "age" ":" type "->" type
    ↓
"*" "age" ":" "integer" "->" "scalar"
```

### 差分マーカー付き型定義

```
+ type NewType {
  name: string
}
```

```
add_remove_marker "type" type_name "{" field_list "}"
    ↓
"+" "type" "NewType" "{" ("name" ":" "string") "}"
```

### 差分マーカーの制約

1. **2重マーカー禁止**
   ```
   + + field: string   # エラー
   ```
   - diff_marker は1つのみ許可

2. **型変更は `*` マーカー時のみ**
   ```
   * field: integer -> scalar   # OK
   field: integer -> scalar     # エラー（マーカーなしで -> は不可）
   + field: integer -> scalar   # エラー（+ で -> は不可）
   ```

3. **`*` は型定義に使用不可**
   ```
   + type NewType { ... }   # OK
   - type OldType { ... }   # OK
   * type Changed { ... }   # エラー
   ```

### バリデーション時の解釈

| マーカー | current（現行） | next（次期） |
|----------|-----------------|--------------|
| なし | フィールド有効 | フィールド有効 |
| `+` | フィールド無視 | フィールド有効 |
| `-` | フィールド有効 | フィールド無視 |
| `*` | 左側の型を使用 | 右側の型を使用 |
