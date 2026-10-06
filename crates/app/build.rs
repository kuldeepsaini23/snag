// Puts the Snag icon and product name into snag.exe (Explorer, taskbar, Start menu).
fn main() {
    println!("cargo:rerun-if-changed=assets/logo/snag.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/logo/snag.ico").set("ProductName", "Snag").set("FileDescription", "Snag — download manager");
        if let Err(e) = res.compile() {
            // A missing resource compiler only costs the exe its icon; the app still builds.
            println!("cargo:warning=no exe icon: {e}");
        }
    }
}
