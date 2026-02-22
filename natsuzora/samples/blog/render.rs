use std::{fs, path::Path};

use natsuzora::Natsuzora;
use serde_json::Value;

fn main() -> natsuzora::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/blog");

    // 1. トップページ
    render_page(
        root.join("index.ntzr"),
        root.join("shared"),
        root.join("data.json"),
        root.join("dist/index.html"),
    )?;

    // 2. プロフィール
    render_page(
        root.join("profile.ntzr"),
        root.join("shared"),
        root.join("data.json"),
        root.join("dist/profile/index.html"),
    )?;

    // 3. 個別記事 (2本)
    render_page(
        root.join("post.ntzr"),
        root.join("shared"),
        root.join("post-component-design-best-practices.json"),
        root.join("dist/posts/component-design-best-practices/index.html"),
    )?;
    render_page(
        root.join("post.ntzr"),
        root.join("shared"),
        root.join("post-include-safety-checklist.json"),
        root.join("dist/posts/include-safety-checklist/index.html"),
    )?;

    // 4. カテゴリ一覧 (例: Engineering)
    render_page(
        root.join("category.ntzr"),
        root.join("shared"),
        root.join("category-engineering.json"),
        root.join("dist/categories/engineering/index.html"),
    )?;

    println!("Blog sample written to {}/dist", root.display());
    Ok(())
}

fn render_page(
    template_path: impl AsRef<Path>,
    include_root: impl AsRef<Path>,
    data_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> natsuzora::Result<()> {
    let template = fs::read_to_string(template_path.as_ref())?;
    let data: Value = serde_json::from_slice(&fs::read(data_path.as_ref())?)?;

    let engine = Natsuzora::parse_with_includes(&template, include_root)?;
    let html = engine.render(data)?;

    if let Some(parent) = output_path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, html)?;
    Ok(())
}
