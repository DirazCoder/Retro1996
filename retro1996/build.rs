use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS is not set");
    
    if target_os == "windows" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");

        let mut res = winres::WindowsResource::new();

        let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
        let icon_path = Path::new(&crate_dir).join("assets").join("icon.ico");

        if icon_path.exists() {
            res.set_icon(icon_path.to_str().expect("Icon path is not valid UTF-8"));
        } else {
            eprintln!("Warning: Icon file not found at {:?}", icon_path);
        }

        let pkg_version = env!("CARGO_PKG_VERSION");
        let parts: Vec<&str> = pkg_version.split('.').collect();
        let major = parts.get(0).unwrap_or(&"0").parse::<u16>().unwrap_or(0);
        let minor = parts.get(1).unwrap_or(&"0").parse::<u16>().unwrap_or(0);
        let patch = parts.get(2).unwrap_or(&"0").parse::<u16>().unwrap_or(0);
        let build = 0;

        let version_string = format!("{}.{}.{}.{}", major, minor, patch, build);
        res.set("FileVersion", &version_string);
        res.set("ProductVersion", &version_string);

        res.set("ProductName", env!("CARGO_PKG_NAME"));
        res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
        let authors = env!("CARGO_PKG_AUTHORS");
        let first_author = authors.split(',').next().unwrap_or(" ").trim();
        res.set("LegalCopyright", first_author);

        res.set("OriginalFilename", "retro1996.exe");
        res.set("InternalName", "retro1996");
        res.set("CompanyName", first_author);

        match res.compile() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error compiling Windows resources: {}", e);
            }
        }
    }
}