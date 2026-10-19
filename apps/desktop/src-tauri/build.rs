use std::path::Path;

/// The Spicetify extension is embedded in the app (see `setup.rs`).
const EXTENSION: &str = "../../../extension/dist/focuslock.js";

fn main() {
    println!("cargo:rerun-if-changed={EXTENSION}");
    assert!(
        Path::new(EXTENSION).is_file(),
        "{EXTENSION} is missing: run `pnpm build` in extension/ first"
    );
    tauri_build::build();
}
