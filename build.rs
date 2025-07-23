#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    use std::env;
    let exe = env::var("CARGO_BIN_NAME").unwrap_or_default();
    let mut res = winres::WindowsResource::new();
    if exe == "TwitchYapBot" {
        res.set_icon("resources/icon/yap_icon_purple.ico");
        res.set("FileDescription", "Twitch Yap Bot");
        res.set("ProductName", "Twitch Yap Bot");
        res.set("OriginalFilename", "TwitchYapBot.exe");
    } else if exe == "YapBotInstaller" {
        res.set_icon("resources/icon/yap_icon_green.ico");
        res.set("FileDescription", "Yap Bot Installer");
        res.set("ProductName", "Yap Bot Installer");
        res.set("OriginalFilename", "YapBotInstaller.exe");
    } else if exe == "YapBotUpdater" {
        res.set_icon("resources/icon/yap_icon_blue.ico");
        res.set("FileDescription", "Yap Bot Updater");
        res.set("ProductName", "Yap Bot Updater");
        res.set("OriginalFilename", "YapBotUpdater.exe");
    }
    res.compile().unwrap();
    
    // Only hide console in release builds
    if std::env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    }
}

#[cfg(not(windows))]
fn main() {} 