# Natsuzora Spec Tests

共通テストケースによる Natsuzora 実装の互換性検証。

## テストケースフォーマット

### 成功ケース

```json
{
  "name": "テスト名",
  "template": "テンプレート文字列",
  "data": { "key": "value" },
  "expected": "期待される出力"
}
```

### エラーケース

```json
{
  "name": "テスト名",
  "template": "テンプレート文字列",
  "data": { "key": "value" },
  "error": "エラータイプ"
}
```

エラータイプ:
- `UndefinedVariable` - 未定義変数
- `TypeError` - 型エラー（stringify不可、each対象が配列でない等）
- `ReservedWordError` - 予約語を変数名として使用
- `ParseError` - 構文エラー
- `ShadowingError` - シャドーイング違反

### インクルードケース

```json
{
  "name": "テスト名",
  "template": "テンプレート文字列",
  "partials": {
    "/path/name": "パーシャル内容"
  },
  "data": { "key": "value" },
  "expected": "期待される出力"
}
```

## テストファイル

| ファイル | 内容 |
|----------|------|
| `basic.json` | 基本的な変数展開、HTMLエスケープ |
| `if_block.json` | 条件分岐（if/else） |
| `each_block.json` | ループ（each）|
| `unsecure.json` | エスケープ無効化 |
| `truthiness.json` | 真偽判定 |
| `stringify.json` | 文字列化 |
| `errors.json` | エラーケース |
| `include.json` | インクルード |

## 実装での使用例

### Ruby

```ruby
require 'json'
require 'natsuzora'

tests = JSON.parse(File.read('basic.json'))
tests['tests'].each do |test|
  result = Natsuzora.render(test['template'], test['data'])
  if test['expected']
    raise "#{test['name']} failed" unless result == test['expected']
  end
rescue => e
  if test['error']
    # エラーが期待される場合は成功
  else
    raise
  end
end
```

### Rust

```rust
use serde_json::{json, Value};
use std::fs;

#[derive(Deserialize)]
struct TestCase {
    name: String,
    template: String,
    data: Value,
    expected: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct TestFile {
    tests: Vec<TestCase>,
}

fn run_tests(file: &str) {
    let content = fs::read_to_string(file).unwrap();
    let test_file: TestFile = serde_json::from_str(&content).unwrap();

    for test in test_file.tests {
        let result = natsuzora::render(&test.template, test.data.clone());

        match (result, test.expected, test.error) {
            (Ok(output), Some(expected), None) => {
                assert_eq!(output, expected, "Test '{}' failed", test.name);
            }
            (Err(_), None, Some(_)) => {
                // Expected error occurred
            }
            _ => panic!("Unexpected result for '{}'", test.name),
        }
    }
}
```
