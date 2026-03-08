use std::process::Command;

/// Thứ tự concat JS quan trọng: core → components → pages → app init
const JS_FILES: &[&str] = &[
    "frontend/js/01-core.js",
    "frontend/js/02-components.js",
    "frontend/js/03-page-status.js",
    "frontend/js/04-page-channels.js",
    "frontend/js/05-page-uart.js",
    "frontend/js/06-page-network.js",
    "frontend/js/07-page-routing.js",
    "frontend/js/08-page-toolbox.js",
    "frontend/js/09-page-system.js",
    "frontend/js/10-app.js",
];
const HTML_TEMPLATE: &str = "frontend/index-template.html";
const HTML_OUTPUT: &str = "html-bundle/embedded_index.html";

fn main() {
    // Build date + time (YYYY-MM-DD HH:MM)
    let date = Command::new("date")
        .args(["+%Y-%m-%d %H:%M"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=BUILD_DATE={}", date);

    // Git commit hash (short)
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=GIT_COMMIT={}", commit);

    // Concat JS files + template → embedded HTML bundle
    let mut js_bundle = String::new();
    for f in JS_FILES {
        js_bundle.push_str(&std::fs::read_to_string(f).expect(f));
        js_bundle.push('\n');
        println!("cargo:rerun-if-changed={}", f);
    }
    let template = std::fs::read_to_string(HTML_TEMPLATE).expect(HTML_TEMPLATE);
    println!("cargo:rerun-if-changed={}", HTML_TEMPLATE);
    let output = template.replace("{{JS_BUNDLE}}", &js_bundle);
    std::fs::write(HTML_OUTPUT, output).expect(HTML_OUTPUT);
}
