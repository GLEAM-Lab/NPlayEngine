fn main() {
    // The CLI embeds the web build. Keep plain `cargo check` working without one.
    let _ = std::fs::create_dir_all("../../dist/arena");
    for d in ["dist", "games", "sdks"] {
        println!("cargo:rerun-if-changed=../../{d}");
    }
}
