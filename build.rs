#[cfg(windows)]
fn main() {
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
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

    // --- Sound array generation ---
    let out_dir = env::var("OUT_DIR").unwrap();
    let sound_dir = Path::new("resources/sound");
    let write_sound_array = |prefix: &str, out_name: &str| {
        let mut entries: Vec<_> = fs::read_dir(sound_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with(prefix) && name.ends_with(".mp3")
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());
        let mut file = File::create(Path::new(&out_dir).join(out_name)).unwrap();
        let static_name = match prefix {
            "DeathScream" => "DEATH_SCREAMS",
            "Angelic" => "ANGELIC_SOUNDS",
            _ => panic!("Unknown prefix"),
        };
        writeln!(file, "pub static {}: [&'static [u8]; {}] = [", static_name, entries.len()).unwrap();
        for entry in &entries {
            let name = entry.file_name().to_string_lossy().to_string();
            writeln!(file, "    include_bytes!(\"../../../../../resources/sound/{}\"),", name).unwrap();
        }
        writeln!(file, "];").unwrap();
    };
    write_sound_array("DeathScream", "death_screams_generated.rs");
    write_sound_array("Angelic", "angelic_sounds_generated.rs");
}

#[cfg(not(windows))]
fn main() {} 