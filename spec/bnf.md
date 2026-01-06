# ミニテンプレート v2.0 BNF（EBNF表記）

## 前提（文字コード）

入力テンプレートは UTF-8 のテキストである。

## 字句（トークン）規則

### (1) 予約記号（固定文字列）

```bnf
OPEN ::= "{["
CLOSE ::= "]}"
HASH ::= "#"
SLASH ::= "/"
GT ::= ">"
EQUAL ::= "="
COMMA ::= ","
EXCLAIM ::= "!"
DASH ::= "-"
DOT ::= "."
LBRACE ::= "{"
```

### (2) キーワード

```bnf
KW_IF ::= "if"
KW_UNLESS ::= "unless"
KW_EACH ::= "each"
KW_AS ::= "as"
KW_UNSECURE ::= "unsecure"
KW_ELSE ::= "else"
KW_TRUE ::= "true"
KW_FALSE ::= "false"
KW_NULL ::= "null"
KW_INCLUDE ::= "include"
```

注: これらのキーワードは識別子として使用できない（予約語）

### (3) 空白

```bnf
WS ::= (" " | "\t" | "\r" | "\n")+
```

### (4) 識別子

```bnf
IDENT_START ::= "A".."Z" | "a".."z"
IDENT_CONT ::= IDENT_START | "0".."9" | "_"
IDENT ::= IDENT_START IDENT_CONT*
```

注: `_` で始まる識別子は禁止（`_` は2文字目以降でのみ使用可能）

### (5) 変数パス（ドット区切り）

```bnf
PATH ::= IDENT ( DOT IDENT )*
```

### (6) include 名（論理名）

```
NAME_SEG_CHAR ::= "A".."Z" | "a".."z" | "0".."9" | "_"
NAME_SEG ::= NAME_SEG_CHAR+
NAME ::= "/" NAME_SEG ("/" NAME_SEG)*
```

### 追加制約（構文外の検証）

NAME は以下を満たす必要がある（満たさない場合は構文エラーとして扱ってよい）

- "//" を含まない（連続スラッシュ禁止）
- "\" を含まない（バックスラッシュ禁止）
- ":" を含まない（Windows パス区切り禁止）
- 空でない（NAME自体が空を許さないが明記）

注: `.` や `..` は NAME_SEG_CHAR に含まれないため、文法上すでに禁止されている

### (7) テキスト

```bnf
TEXT ::= （OPEN を開始としない任意の文字列の最長一致）
```

注: 実装では通常「次の '{[' まで」を TEXT として切り出す

### (8) タグ開閉（空白制御対応）

```bnf
TAG_OPEN ::= OPEN DASH?
TAG_CLOSE ::= DASH? CLOSE
```

注:

- `{[` と `-` の間、および `-` と `]}` の間に空白は許可されない
- `{[-` と `#`/`>`/`/` の間にも空白は許可されない（`{[-#if`、`{[->`、`{[-/if`）
- `{[-`: 直前の TEXT の末尾から行頭までの空白を削除（行が空白のみの場合）
- `-]}`: 直後の TEXT の先頭から行末までの空白と改行を削除（行が空白のみの場合）
- Lexer レベルで処理され、AST ノードとしては存在しない
- 行に非空白文字が含まれる場合は何もしない

## 構文（文法）規則

### 2.1 ルート

```bnf
TEMPLATE ::= NODE*

NODE ::= TEXT
  | VAR_NODE
  | IF_BLOCK
  | UNLESS_BLOCK
  | EACH_BLOCK
  | UNSECURE_BLOCK
  | INCLUDE_NODE
```

### 2.2 変数展開

```bnf
VAR_NODE ::= TAG_OPEN VAR TAG_CLOSE
VAR ::= WS? PATH WS?
```

### 2.3 if ブロック

```bnf
IF_BLOCK ::= IF_OPEN NODE* IF_ELSE_PART? IF_CLOSE
IF_OPEN ::= TAG_OPEN HASH WS? KW_IF WS+ EXPR WS? TAG_CLOSE
IF_CLOSE ::= TAG_OPEN SLASH WS? KW_IF WS? TAG_CLOSE
IF_ELSE_PART ::= ELSE_OPEN NODE*
ELSE_OPEN ::= TAG_OPEN HASH WS? KW_ELSE WS? TAG_CLOSE
```

### 2.4 unless ブロック

```bnf
UNLESS_BLOCK ::= UNLESS_OPEN NODE* UNLESS_CLOSE
UNLESS_OPEN ::= TAG_OPEN HASH WS? KW_UNLESS WS+ EXPR WS? TAG_CLOSE
UNLESS_CLOSE ::= TAG_OPEN SLASH WS? KW_UNLESS WS? TAG_CLOSE
```

### 2.5 each ブロック

```bnf
EACH_BLOCK ::= EACH_OPEN NODE* EACH_CLOSE
EACH_OPEN ::= TAG_OPEN HASH WS? KW_EACH WS+ EXPR WS+ KW_AS WS+ IDENT EACH_INDEX? WS? TAG_CLOSE
EACH_INDEX ::= WS? COMMA WS? IDENT
EACH_CLOSE ::= TAG_OPEN SLASH WS? KW_EACH WS? TAG_CLOSE
```

### 2.6 unsecure ブロック

```bnf
UNSECURE_BLOCK ::= UNSECURE_OPEN NODE* UNSECURE_CLOSE
UNSECURE_OPEN ::= TAG_OPEN HASH WS? KW_UNSECURE WS? TAG_CLOSE
UNSECURE_CLOSE ::= TAG_OPEN SLASH WS? KW_UNSECURE WS? TAG_CLOSE
```

### 2.7 include（パーシャル）

```bnf
INCLUDE_NODE ::= TAG_OPEN INCLUDE TAG_CLOSE
INCLUDE ::= GT WS? NAME INCLUDE_ARGS? WS?

INCLUDE_ARGS ::= (WS+ INCLUDE_ARG)+
INCLUDE_ARG ::= IDENT WS? EQUAL WS? PATH
```

注:

- include 引数は「1つ以上の空白」で区切られる
- カンマ区切りは存在しない
- include の value は PATH のみ（リテラル、式、関数呼び出しは存在しない）

### 2.8 コメント

```bnf
COMMENT ::= TAG_OPEN EXCLAIM COMMENT_CONTENT TAG_CLOSE
COMMENT_CONTENT ::= （CLOSE を含まない任意の文字列）
```

注:

- コメントは Lexer レベルで完全にスキップされ、AST ノードとしては存在しない
- そのため NODE の選択肢には含まれない
- 複数行のコメントも可能

### 2.9 デリミタエスケープ

```bnf
DELIMITER_ESCAPE ::= OPEN LBRACE CLOSE
```

注:

- `{[{]}` は厳密に 5 文字で固定（空白の挿入は一切許容されない）
- 空白制御との併用は不可（`{[-{]}` や `{[{-]}` は存在しない）
- Lexer レベルで処理され、TEXT として `{[` を出力する
- AST ノードとしては存在しない（TEXT ノードに変換される）

### 2.10 式（v1.4 では PATH のみ）

```bnf
EXPR ::= PATH
```

## 構文外（セマンティクスに属する）追加制約

### (1) ブロックの対応

- IF_OPEN は対応する IF_CLOSE で閉じなければならない
- UNLESS_OPEN は対応する UNLESS_CLOSE で閉じなければならない
- EACH_OPEN は対応する EACH_CLOSE で閉じなければならない
- UNSECURE_OPEN は対応する UNSECURE_CLOSE で閉じなければならない
- ブロックはネスト可能
- 異なる種類で閉じるのはエラー（例: {[#if ...]} ... {[/each]}）

### (2) each の as 必須

- each は必ず「as IDENT」を伴う（BNFで強制済み）

### (3) include 引数の重複

- 同一 INCLUDE の中で IDENT（key）の重複は禁止（静的検証または実行時エラー）

### (4) include 名 NAME の禁止パターン

- 「//」「\」「:」が含まれる場合はエラー（前述）
- 「.」「..」は NAME_SEG_CHAR に含まれないため文法上禁止

### (5) 予約語と IDENT の関係

- 全てのキーワード（if, unless, each, as, unsecure, else, true, false, null, include）は識別子として使用不可
- これらは文脈に関わらず予約語として扱われる

## 実装メモ（非規範）

- 字句解析では TEXT と `{[ ... ]}` のセクションを交互に切り出すと実装しやすい
- `{[ ... ]}` の先頭記号で分岐すると判定が容易:
  - '#' ならブロック開始
  - '/' ならブロック終了
  - '>' なら include
  - '!' ならコメント（CLOSE までスキップ）
  - '-' なら空白制御（直前の空白を行頭まで削除）
  - '{' ならデリミタエスケープ（`{[` をリテラル出力）
  - それ以外は VAR
- '-]}' は空白制御（直後の空白を行末まで削除）
