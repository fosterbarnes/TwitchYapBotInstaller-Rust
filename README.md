# TwitchYapBotInstaller-Rust
This bot reads everything in your twitch chat and learns how to speak. Just type "!yap" in chat. This is a Windows only application.

![yap example](https://github.com/user-attachments/assets/0e3da20f-a635-4749-a04a-83609ac17a40)

## How to install
- Download and install both x86 & x64 versions of [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170)
  - [vc_redist.x86.exe](https://aka.ms/vs/17/release/vc_redist.x86.exe)
  - [vc_redist.x64.exe](https://aka.ms/vs/17/release/vc_redist.x64.exe)
- [Download the latest release](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases/download/v5.0.4/Yap.Bot.Installer.v5.0.4.exe)
- After it's installed, run the shortcut from your desktop or start menu app list. Happy yappin'
- The install will live at `YourUserName\AppData\Roaming\YapBot`. User specified install locations are planned for the future

## Notes about v5.0.3 and newer
1. If you encounter this error when launching Yap Bot:
   
  ![errorScreenshot](https://i.postimg.cc/NMtTnkmt/Virtual-Box-VM-f5dcj-WRF3-O.png)

- Download and install [vc_redist.x86.exe](https://aka.ms/vs/17/release/vc_redist.x86.exe) then restart the app

2. Starting in v5.0.3, Windows may incorrectly flag the installer as a virus. You may need to allow your browser to download it. If Windows blocks it from running, click the windows security notification that pops up, then allow it to run. More info on setting exclusions in Windows Security if needed: https://www.elevenforum.com/t/add-or-remove-exclusions-for-microsoft-defender-antivirus-in-windows-11.8797/

	This is a known problem with the app, but not much can be done about it. Fixing this false flag would mean paying hundreds of dollars a year for code-signing. The component that's most likely triggering antivirus is [traymond-tcp](https://github.com/fosterbarnes/traymond-tcp). Yap Bot is built on the [egui](https://github.com/emilk/egui) library, which does not have the ability to natively minimize windows to the system tray. To be able to add this feature, I had to fork the original build of [traymond](https://github.com/fcFn/traymond) and edit it to be able to communicate with Yap Bot. The original traymond waits for a set keyboard combination from the user, then minimizes the selected window to tray when those keys are pressed. Because of this keyboard monitoring, some anti-viruses interpret this as malicious and attempt to block it. In it's current re-worked state, we don't even use the key-combo function, and just use it to receive commands from Yap Bot, then minimize Yap Bot to tray, but this original code remains in the project.

	All of that being said, always exercise caution when running unknown apps from github. This app and traymond-tcp are completely open source, so feel free to go through the code and build for yourself if you're worried about anything malicious.

## How it works
- Train Yap Bot by just typing in chat. All chatter's messages will be added to the database
- When Yap Bot is run, it'll use previous chat messages to formulate a new, randomized message
- In addition to being able to run the bot with "!yap", you can also give it a starting point for the sentence it generates. e.g. "!yap dingus"
- These messages can only start with a word that has previously started a chat message, so don't expect every word to work unless it has been indexed
- You can "train" the bot by feeding it chat messages with a starting word you'd like to add with the database. e.g. "dingus poop fart butt"

## How it's made
- The core script is built on [TwitchMarkovChain](https://github.com/fosterbarnes/TwitchMarkovChain) in python. Many, many details and "hidden" options are listed on this repo
- The installer, client app and updater are built using Rust

## Components
- `Yap Bot Installer v5.0.4.exe` is responsible for making sure python and necessary dependencies are installed, installing the included binaries (`TwitchYapBot.exe` and `YapBotUpdater.exe`) to `User\AppData\Roaming\YapBot`
- `TwitchYapBot.exe` is responsible for running the python chat bot, (`TwitchMarkovChain.py`) showing its output, shutting it down, restarting it, and editing its settings. In Yap Bot's previous rendition, these settings had to be changed manually in a .json file
- `YapBotUpdater.exe` responsible for automatically updating `TwitchYapBot.exe` to the newest version
- `traymond-tcp.exe` is responsible for minimizing the app to the system tray https://github.com/fosterbarnes/traymond-tcp

## Screenshots
Yap Bot Installer:

<img width="800" height="610" alt="Yap Bot Installer v5.0.1" src="https://github.com/user-attachments/assets/835e3973-5907-44b6-9071-61347f4ea31d" />



TwitchYapBot:

<img width="800" height="547" alt="TwitchYapBotv5.0.3" src="https://i.postimg.cc/vHcQfby5/Twitch-Yap-Bot-Hrqd-I8z-Ipq.png" />



YapBotUpdater:

<img width="400" height="112" alt="YapBotUpdaterv5.0.1" src="https://github.com/user-attachments/assets/2fef4e40-87e0-4f51-be38-ac98bd5dcf58" />

## Support

If you have any issues, create an issue from the [Issues](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/issues) tab and I will get back to you as quickly as possible.

If you'd like to support me, follow me on twitch:
https://www.twitch.tv/fosterbarnes

or if you're feeling generous drop a donation:
https://coff.ee/fosterbarnes
