//! Release archives must carry all bundled font notices.
use std::{fs, path::Path};

#[test]
fn release_packages_bundled_font_notices() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    for name in ["INTER-LICENSE.txt", "GEIST-LICENSE.txt", "GEIST-SOURCE.txt"] {
        assert!(root.join("assets").join(name).is_file());
        for step in [
            "Package (Unix)",
            "Package (Windows)",
            "Package Unix editor",
            "Package Windows editor",
        ] {
            let marker = format!("      - name: {step}\n");
            let body = workflow
                .split_once(&marker)
                .unwrap_or_else(|| panic!("missing release step: {step}"))
                .1
                .split("      - ")
                .next()
                .unwrap();
            assert!(
                body.contains(&format!("assets/{name}")),
                "{step} must include {name}"
            );
        }
        let server = fs::read_to_string(root.join("scripts/package-server-bundle.sh")).unwrap();
        assert!(
            server.contains(&format!("assets/{name}")),
            "server bundle must include {name}"
        );
    }
}
