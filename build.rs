#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon/yap_icon_green.ico");
    res.compile().unwrap();
    
    // Only hide console in release builds
    if std::env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    }
}

#[cfg(not(windows))]
fn main() {} 