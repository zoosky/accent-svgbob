//! Regenerates the parity goldens under tests/parity/.
//! Run: cargo run -p svgbob --example gen_parity
use std::fs;

fn main() {
    let data = std::path::Path::new("crates/svgbob/test_data");
    let out = std::path::Path::new("crates/svgbob/tests/parity");
    fs::create_dir_all(out).unwrap();
    for entry in fs::read_dir(data).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("bob") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let ascii = fs::read_to_string(&path).unwrap();
        fs::write(
            out.join(format!("{stem}.pretty.svg")),
            svgbob::to_svg_string_pretty(&ascii),
        )
        .unwrap();
        fs::write(
            out.join(format!("{stem}.compressed.svg")),
            svgbob::to_svg_string_compressed(&ascii),
        )
        .unwrap();
        let settings = svgbob::Settings {
            include_backdrop: false,
            include_styles: false,
            include_defs: false,
            ..Default::default()
        };
        fs::write(
            out.join(format!("{stem}.bare.svg")),
            svgbob::to_svg_with_settings(&ascii, &settings),
        )
        .unwrap();
    }
    println!("goldens written");
}
