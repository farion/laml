use std::fs;
use std::path::Path;

#[test]
fn rust_source_files_do_not_exceed_500_lines() {
    fn check(directory: &Path) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                check(&path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let content = fs::read_to_string(&path).unwrap();
                let lines = content.lines().count();
                assert!(
                    lines <= 500,
                    "{} has {lines} lines (maximum: 500)",
                    path.display()
                );
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    check(&root.join("src"));
    check(&root.join("tests"));
}
