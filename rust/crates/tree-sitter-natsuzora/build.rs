use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let grammar_dir = manifest_dir
        .parent()
        .expect("crates dir")
        .parent()
        .expect("rust dir")
        .parent()
        .expect("workspace root")
        .join("tree-sitter")
        .join("src");

    cc::Build::new()
        .include(grammar_dir.join("tree_sitter"))
        .file(grammar_dir.join("parser.c"))
        .warnings(false)
        .compile("tree-sitter-natsuzora");

    println!(
        "cargo:rerun-if-changed={}",
        grammar_dir.join("parser.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        grammar_dir.join("node-types.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        grammar_dir.join("grammar.json").display()
    );
}
