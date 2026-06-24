use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let syntax_dir = Path::new("assets/syntaxes");
    let syntax_dump = Path::new(&out_dir).join("syntax_set.bin");

    let mut builder = syntect::parsing::SyntaxSet::load_defaults_newlines().into_builder();

    if syntax_dir.exists() {
        builder.add_from_folder(syntax_dir, true).unwrap();

        println!("cargo:rerun-if-changed=assets/syntaxes");
        for entry in std::fs::read_dir(syntax_dir).unwrap() {
            let entry = entry.unwrap();
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    let syntax_set = builder.build();
    syntect::dumps::dump_to_uncompressed_file(&syntax_set, syntax_dump).unwrap();
}
