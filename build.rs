fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set("ProductName", "Dev Dynamic Island");
        res.set("FileDescription", "Developer Dynamic Island for Windows");
        res.set("LegalCopyright", "Copyright (C) 2026");
        let _ = res.compile();
    }
}
