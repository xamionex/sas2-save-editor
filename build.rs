fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName",     "SaS2 Save Editor");
        res.set("FileDescription", "Salt and Sacrifice Save Editor");
        res.set("CompanyName",     "xamionex");
        res.set("LegalCopyright",  "© 2026 xamionex");
        res.set("OriginalFilename","sas2-save-editor.exe");
    }
}
