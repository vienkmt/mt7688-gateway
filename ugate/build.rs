use std::process::Command;

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

    // Concat JS files + template → embedded_index.html
    let js_files = [
        "src/js/01-core.js",
        "src/js/02-helpers.js",
        "src/js/03-page-status.js",
        "src/js/04-page-config.js",
        "src/js/05-page-uart.js",
        "src/js/06-page-network.js",
        "src/js/07-page-routing.js",
        "src/js/08-page-toolbox.js",
        "src/js/09-page-system.js",
        "src/js/10-ws.js",
    ];
    let mut js_bundle = String::new();
    for f in &js_files {
        js_bundle.push_str(&std::fs::read_to_string(f).expect(f));
        js_bundle.push('\n');
        println!("cargo:rerun-if-changed={}", f);
    }
    let template = std::fs::read_to_string("src/index-template.html")
        .expect("src/index-template.html");
    println!("cargo:rerun-if-changed=src/index-template.html");
    let output = template.replace("{{JS_BUNDLE}}", &js_bundle);
    std::fs::write("src/embedded_index.html", output)
        .expect("write embedded_index.html");
}
