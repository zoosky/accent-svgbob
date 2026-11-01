//! Byte-parity against the sauron-rendered output.
//!
//! The goldens under `tests/parity/` were generated at the last revision
//! that rendered through sauron 0.61, by `examples/gen_parity.rs`. Every
//! `.bob` fixture is rendered through the three public entry points and
//! must match byte for byte. Regenerate deliberately with
//! `cargo run -p svgbob --example gen_parity` and review the diff.

use std::{fs, path::Path};

fn fixtures() -> Vec<(String, String)> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data");
    let mut fixtures: Vec<(String, String)> = fs::read_dir(data)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("bob") {
                return None;
            }
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            Some((stem, fs::read_to_string(path).unwrap()))
        })
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(), "no .bob fixtures found");
    fixtures
}

fn golden(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/parity")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing golden {}: {e}", path.display()))
}

#[test]
fn pretty_output_matches_sauron_golden() {
    for (stem, ascii) in fixtures() {
        let rendered = svgbob::to_svg_string_pretty(&ascii);
        assert_eq!(
            rendered,
            golden(&format!("{stem}.pretty.svg")),
            "pretty output diverges for {stem}.bob"
        );
    }
}

#[test]
fn compressed_output_matches_sauron_golden() {
    for (stem, ascii) in fixtures() {
        let rendered = svgbob::to_svg_string_compressed(&ascii);
        assert_eq!(
            rendered,
            golden(&format!("{stem}.compressed.svg")),
            "compressed output diverges for {stem}.bob"
        );
    }
}

#[test]
fn bare_output_matches_sauron_golden() {
    for (stem, ascii) in fixtures() {
        let mut settings = svgbob::Settings::default();
        settings.include_backdrop = false;
        settings.include_styles = false;
        settings.include_defs = false;
        let rendered = svgbob::to_svg_with_settings(&ascii, &settings);
        assert_eq!(
            rendered,
            golden(&format!("{stem}.bare.svg")),
            "bare output diverges for {stem}.bob"
        );
    }
}
