# TwitchYapBotInstaller-Rust
This bot reads everything in your twitch chat and learns how to speak. Just type "!yap" in chat. This is a Windows only application.

![yap example](https://github.com/user-attachments/assets/0e3da20f-a635-4749-a04a-83609ac17a40)

## How to install
- Download and install the latest version of [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
- [Download the latest release](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases/download/v5.0.0/Yap.Bot.Installer.v5.0.0.exe), run the installer, and follow the on screen instructions. Happy yappin'

## How it works
- Train Yap Bot by having people type in chat. When Yap Bot is run, it'll use previous chat messages to formulate a new, randomized message.
- In addition to being able to run the bot with "!yap", you can also give it a starting point for the sentance it generates. e.g. "!yap dingus".
- These messages can only start with a word that has previously started a chat message, so don't expect every word to work unless it has been indexed.
- You can "train" the bot by feeding it chat messages with a starting word you'd like to add with the database. e.g. "dingus poop fart butt"
