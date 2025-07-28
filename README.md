# TwitchYapBotInstaller-Rust
This bot reads everything in your twitch chat and learns how to speak. Just type "!yap" in chat. This is a Windows only application.

![yap example](https://github.com/user-attachments/assets/0e3da20f-a635-4749-a04a-83609ac17a40)

## How to install
- Download and install the latest version of [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
[Download the latest release](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases/download/v5.0.2/Yap.Bot.Installer.v5.0.2.exe)
- After it's installed, run the shortcut from your desktop or start menu app list. Happy yappin'
- The install will live at `YourUserName\AppData\Roaming\YapBot`. User specified install locations are planned for the future

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
- `Yap Bot Installer v5.0.2.exe` is responsible for making sure python and necessary dependencies are installed, installing the included binaries (`TwitchYapBot.exe` and `YapBotUpdater.exe`) to `User\AppData\Roaming\YapBot`
- `TwitchYapBot.exe` is responsible for running the python chat bot, (`TwitchMarkovChain.py`) showing its output, shutting it down, restarting it, and editing its settings. In Yap Bot's previous rendition, these settings had to be changed manually in a .json file
- `YapBotUpdater.exe` responsible for automatically updating `TwitchYapBot.exe` to the newest version

## Support
If you have any issues, create an issue from the [Issues](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/issues) tab and I will get back to you as quickly as possible.

If you'd like to support me, follow me on twitch:
https://www.twitch.tv/fosterbarnes

or if you're feeling generous drop a donation:
https://coff.ee/fosterbarnes

## Screenshots
Yap Bot Installer:

<img width="800" height="610" alt="Yap Bot Installer v5 0 1" src="https://github.com/user-attachments/assets/835e3973-5907-44b6-9071-61347f4ea31d" />


TwitchYapBot:

<img width="800" height="547" alt="TwitchYapBotv5 0 1" src="https://github.com/user-attachments/assets/3b9df747-2817-4a9c-9cd8-4f44c6b54cd3" />


YapBotUpdater:

<img width="400" height="112" alt="YapBotUpdaterv5 0 1" src="https://github.com/user-attachments/assets/2fef4e40-87e0-4f51-be38-ac98bd5dcf58" />
