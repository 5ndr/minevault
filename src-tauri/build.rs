use std::fs;
use std::path::Path;

fn main() {
    // Ensure frontend distribution directory exists so cargo check / build succeeds
    // even if frontend hasn't been built yet in an offline or headless environment.
    let dist_dir = Path::new("../dist");
    if !dist_dir.exists() {
        let _ = fs::create_dir_all(dist_dir);
        let index_html = dist_dir.join("index.html");
        if !index_html.exists() {
            let _ = fs::write(
                &index_html,
                "<!doctype html><html><head><meta charset=\"UTF-8\"/><title>MineVault</title></head><body><div id=\"root\"></div></body></html>",
            );
        }
    }

    tauri_build::build()
}
