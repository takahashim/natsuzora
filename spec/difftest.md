# Natsuzora 差分テスト仕様（Differential Testing）

バージョン: 0.1

## 目的

Ruby 実装と Rust 実装が、同一のテンプレート＋データから完全に同一の出力（または同一種別のエラー）を生成することを検証できるようにする。
そのため、自動生成した大量の入力で継続的に検証するしくみを導入する。

既存の共有テスト（`tests/*.json`）は人間が書いた例ベースのテストであり、仕様の代表点を固定する役割を持つ。差分テストはその補完であり、片方の実装をもう片方のオラクル（正解判定器）として使うことで、期待出力を人間が書くことなく入力空間を広く探索する。

## 用語

| 用語 | 意味 |
|------|------|
| ドライバ | テストケースを生成し、両ワーカーに送り、結果を比較するプログラム（Rust / proptest） |
| ワーカー | 1つの実装をラップし、ケースを受け取ってレンダリング結果を返すプロセス |
| 乖離（divergence） | 両実装の結果が比較規則の下で一致しないこと。すべての乖離はバグ（実装または仕様の） |

## アーキテクチャ

```
ドライバ (Rust, proptest)
  ├── ケース生成（AST → テンプレート文字列 + データ）
  ├── Rust 実装: 同一プロセス内で natsuzora::render を直接呼ぶ
  ├── Ruby 実装: ワーカープロセス 1 本を起動し、JSONL プロトコルでパイプ越しに送受信
  └── 比較・shrink・レポート
```

- 生成器はドライバ（Rust 側）に一元化する。各実装に生成器を重複保守しない。
- Ruby のプロセス起動コストを避けるため、ワーカーは起動しっぱなしでケースをストリーム処理する。1 ケース 1 プロセスは禁止。

## ワーカープロトコル（JSONL）

ワーカーは stdin から 1 行 1 リクエスト（JSON）を読み、stdout に 1 行 1 レスポンスを書く。リクエストの順序とレスポンスの順序は一致させること（`id` は照合と診断用）。

### リクエスト

```json
{"id": 1, "template": "Hello, {[ name ]}!", "data": {"name": "Alice"}}
{"id": 2, "template": "{[ include \"/p\" ]}", "data": {}, "partials": {"/p": "partial body"}}
```

| フィールド | 必須 | 内容 |
|------------|------|------|
| `id` | ✓ | ケース識別子（整数） |
| `template` | ✓ | テンプレート文字列 |
| `data` | ✓ | レンダリング用データ（JSON オブジェクト） |
| `partials` | - | include 用パーシャル。キーはパス、値はテンプレート文字列 |

`partials` が指定された場合、ワーカーはそれを一時ディレクトリに実体化し、`include_root` として実装に渡す（`tests/*.json` の include ケースと同じ扱い）。

### レスポンス

```json
{"id": 1, "ok": true, "output": "Hello, Alice!"}
{"id": 2, "ok": false, "error": "UndefinedVariable"}
```

- 成功: `{"id", "ok": true, "output": <文字列>}`
- 失敗: `{"id", "ok": false, "error": <正規化エラー型>}`
- ワーカー自体のクラッシュ・パニック・タイムアウトは「失敗」ではなくハーネスエラーとして扱い、即座にテスト全体を fail させる（実装はいかなる入力でもクラッシュしてはならない、が前提の性質）。

### エラー型の正規化

エラーは型のみ比較し、メッセージ・位置情報は比較しない。各実装の例外／enum を以下の正規名に写像する。

| 正規名 | Ruby | Rust |
|--------|------|------|
| `ParseError` | `Natsuzora::ParseError`（サブクラスの `LexerError` を含む） | `ParseError` |
| `ReservedWordError` | `Natsuzora::ReservedWordError` | `ParseError` のうち予約語起因（※） |
| `UndefinedVariable` | `Natsuzora::UndefinedVariableError` | `UndefinedVariable` |
| `TypeError` | `Natsuzora::TypeError` | `TypeError` |
| `ShadowingError` | `Natsuzora::ShadowingError` | `ShadowingError` |
| `IncludeError` | `Natsuzora::IncludeError` | `IncludeError` |

※ Rust は予約語エラーを `ParseError`（メッセージに `reserved word` を含む）として返す。既存の共有テストランナー `rust/crates/natsuzora/tests/spec_tests.rs` の `error_type_matches` が同じ判定を行っており、Ruby ワーカー側もこれと対称な写像を実装する。実装間でエラー分類の粒度が異なる箇所（LexerError／ReservedWordError の親子関係など）は、このマッピング表が単一の真実である。実装を変えるのではなく表を仕様として固定し、`tests/errors.json` の既存エラー型と整合させること。マッピング不能なエラーが出た場合はハーネスエラーとする。

## 比較規則

1 ケースにつき、以下のいずれかでなければ乖離（＝テスト失敗）。

1. 両方成功: 出力文字列が UTF-8 バイト列として完全一致
2. 両方失敗: 正規化エラー型が一致
3. それ以外（片方のみ失敗、出力不一致、エラー型不一致）はすべて乖離

## 入力生成（PBT）

### 生成器の方針

ランダムバイト列ではなく、`spec/bnf.md` に基づく AST 生成 → テンプレート文字列化で文法的に意味のあるテンプレートを作る。
生成対象は `tests/*.json` のカテゴリを網羅するようにする。

- 変数展開（単純名・ドットパス・ネスト深さ 1〜5）
- `if` / `unless` / `else`
- `each ... as ...`（ネスト含む）
- `unsecure`
- コメント、デリミタエスケープ、空白制御（`{[-` / `-]}`）
- `include`（`partials` 同時生成、ネスト include 含む）

### データ生成

テンプレート生成後、テンプレート中の変数参照を収集し、2 つのモードでデータを作る。

- 整合モード（既定・約 70%）: 参照される全変数を定義する。値は string（空文字・HTML 特殊文字 `& < > " '`・非 ASCII・改行を含む）、integer（0、負数、±(2⁵³−1) 境界）、boolean、null、array（空含む）、object（空含む）から型注釈に応じて選ぶ。文字列化位置には String/Integer/null、`each` 対象には array、条件位置には任意型。
- 故障注入モード（約 30%）: 上記から 1 箇所を壊す — 変数の未定義化、文字列化位置への Boolean/Array/Object、`each` 対象の非配列化、予約語・`_` 始まり・`@` 含み識別子の使用、シャドーイング、include 循環。両実装が同じエラー型で落ちることを検証するため。

データに浮動小数点数は生成しない（言語仕様で文字列化対象外、Subaru 仕様でも禁止）。

### shrinking

乖離発見時、proptest の shrinking により最小再現ケース（テンプレート＋データ＋partials）まで自動縮小し、再現可能な JSON として artifact に保存すること。

## バイトレベル fuzzing（補助）

`cargo-fuzz` による Rust 実装単体のファジングを補助として行う。

- 性質: 任意のバイト列入力に対しパニックしないこと（`Result` で返る分には正常）
- 差分比較は行わない（Ruby を fuzz ループに入れるのはスループット上非現実的）
- クラッシュ・興味深い入力が見つかった場合は、差分テストのコーパス（固定ケース集）へ追加する

## 乖離発見時のワークフロー

1. shrink 済みの最小ケースを確認し、どちらの実装が仕様（`spec/spec.md`）に対して誤っているか判定する
2. 仕様が曖昧で判定できない場合は、先に `spec/spec.md` を改訂する（CLAUDE.md の原則どおり）
3. 最小ケースを `tests/*.json` の該当カテゴリにゴールデンケースとして昇格する
4. 誤っている実装を修正し、共有テスト＋差分テストの双方が通ることを確認する

## CI 運用

| トリガ | 内容 |
|--------|------|
| PR / push | 固定シード・512 ケースの PBT（数秒〜数十秒で完了すること） |
| 夜間（cron） | ランダムシード・時間制（15 分）の PBT ＋ cargo-fuzz。乖離時は最小ケースを artifact 保存し issue 化 |

固定シード実行は再現性のため、シード値をログに必ず出力する。

## 実装配置

| 成果物 | 場所 |
|--------|------|
| ドライバ＋生成器＋Rust 側比較 | `rust/crates/natsuzora-difftest`（proptest、`natsuzora` crate を直接依存） |
| Ruby ワーカー CLI | `ruby/exe/natsuzora-difftest-worker`（gem 同梱、`Natsuzora.render` をラップ） |
| fuzz ターゲット | `rust/fuzz/`（cargo-fuzz 標準配置） |
| 固定コーパス | `tests/difftest-corpus/*.jsonl`（fuzz・過去乖離由来のケース） |

## 実行方法

### 前提

- Ruby 側の依存関係がインストール済みであること（`cd ruby && bundle install`）。ドライバは Ruby ワーカーを `ruby/` ディレクトリで `bundle exec` 経由で起動する。
- Rust toolchain（workspace の MSRV 以上）。

### 基本の実行

```bash
cd rust
cargo test -p natsuzora-difftest          # 512 ケース（数秒）
```

ケース数は `PROPTEST_CASES` 環境変数で変更できる。

```bash
PROPTEST_CASES=8192 cargo test -p natsuzora-difftest --test differential
```

### 乖離発見時

- 失敗時は shrink 済みの最小ケース（template / data / partials の JSON）がテスト出力に表示される。
- proptest が失敗シードを `rust/crates/natsuzora-difftest/tests/differential.proptest-regressions` に保存する。このファイルはコミットし、以後のすべての実行で自動再生される。
- 単一ケースの再現・診断には repro ツールを使う（両実装の結果を並べて表示する）:

```bash
cd rust
cargo run -p natsuzora-difftest --example repro <<'EOF'
{"template": "Hi {[ n ]}", "data": {"n": "x"}, "partials": {}}
EOF
# rust: Output("Hi x")
# ruby: Output("Hi x")
```

### Ruby ワーカー

- ワーカー単体の動作確認は JSONL を直接流す:

```bash
cd ruby
echo '{"id":1,"template":"Hi {[ n ]}","data":{"n":"x"}}' | bundle exec ruby -Ilib exe/natsuzora-difftest-worker
```

- ドライバからの起動コマンドは環境変数 `NATSUZORA_DIFFTEST_RUBY_CMD` で上書きできる（空白区切り、リポジトリルートで実行される）。例: `NATSUZORA_DIFFTEST_RUBY_CMD="ruby -Iruby/lib ruby/exe/natsuzora-difftest-worker"`

## 非目標

- パフォーマンス比較（スループット差は検証対象外）
- エラーメッセージ文言・位置情報の一致
- 浮動小数点数の取り扱い（言語仕様で対象外）
- Subaru（contract）の差分テスト（必要なら別仕様として定義する）
