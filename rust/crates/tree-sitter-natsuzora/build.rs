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

    let mut build = cc::Build::new();
    build
        .include(grammar_dir.join("tree_sitter"))
        .file(grammar_dir.join("parser.c"))
        .warnings(false);

    // Include external scanner if it exists
    let scanner_path = grammar_dir.join("scanner.c");
    if scanner_path.exists() {
        build.file(&scanner_path);
        println!(
            "cargo:rerun-if-changed={}",
            scanner_path.display()
        );
    }

    build.compile("tree-sitter-natsuzora");

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
