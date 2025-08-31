# TwitchYapBotInstaller-Rust
This bot reads everything in your twitch chat and learns how to speak. Just type "!yap" in chat. This is a Windows only application.

![yap example](https://github.com/user-attachments/assets/0e3da20f-a635-4749-a04a-83609ac17a40)

## How to install
- Download and install both x86 & x64 versions of [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170)
  - [vc_redist.x86.exe](https://aka.ms/vs/17/release/vc_redist.x86.exe)
  - [vc_redist.x64.exe](https://aka.ms/vs/17/release/vc_redist.x64.exe)
- [Download the latest release](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases/download/v5.1.0/Yap.Bot.Installer.v5.1.0.exe)
- After it's installed, run the shortcut from your desktop or start menu app list. Happy yappin'
- The install will live at `YourUserName\AppData\Roaming\YapBot`. User specified install locations are planned for the future

## How it works
- Train Yap Bot by just typing in chat. All chatter's messages will be added to the database
- This app runs locally without communicating with AI models or large language models (LLMs)
- When Yap Bot is run, it'll use previous chat messages to formulate a new, randomized message
- In addition to being able to run the bot with "!yap", you can also give it a starting point for the sentence it generates. e.g. "!yap dingus"
- These messages can only start with a word that has previously started a chat message, so don't expect every word to work unless it has been indexed
- You can "train" the bot by feeding it chat messages with a starting word you'd like to add with the database. e.g. "dingus poop fart butt"


## How it's made
- The core script is built on [TwitchMarkovChain](https://github.com/fosterbarnes/TwitchMarkovChain) in python. Many, many details and "hidden" options are listed on this repo
- The installer, client app,updater & tray app are built using [Rust](https://www.rust-lang.org)

## Components
- `Yap Bot Installer v5.1.0.exe` is responsible for making sure python and necessary dependencies are installed to `User\AppData\Roaming\YapBot`
    - Included binaries: `TwitchYapBot.exe`, `YapBotUpdater.exe` & `YapBotTray.exe`
	- Included files & folders: `TwitchMarkovChain` & `yap_icon_purple.ico`
- `TwitchYapBot.exe` is responsible for running the python chat bot, (`TwitchMarkovChain.py`) showing its output, shutting it down, restarting it, and editing its settings. In Yap Bot's previous rendition, these settings had to be changed manually in a .json file
- `YapBotUpdater.exe` responsible for automatically updating `TwitchYapBot.exe` to the newest version
- `YapBotTray.exe` is a standalone binary that runs Yap Bot in the Windows system tray. We switch between `TwitchYapBot.exe` & `YapBotTray.exe` based on the user's settings, or when they press the "to tray" button in the GUI

## Screenshots
Yap Bot Installer:

<img width="800" height="610" alt="Yap Bot Installer" src="https://github.com/user-attachments/assets/835e3973-5907-44b6-9071-61347f4ea31d" />



TwitchYapBot:

<img width="800" height="547" alt="TwitchYapBot" src="https://i.postimg.cc/vHcQfby5/Twitch-Yap-Bot-Hrqd-I8z-Ipq.png" />



YapBotUpdater:

<img width="400" height="112" alt="YapBotUpdater" src="https://github.com/user-attachments/assets/2fef4e40-87e0-4f51-be38-ac98bd5dcf58" />




YapBotTray:

<img width="302" height="213" alt="YapBotTray" src="https://i.postimg.cc/9QNJ2mzd/JGj-Z4ccaf6.png" />




## Support

If you have any issues, create an issue from the [Issues](https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/issues) tab and I will get back to you as quickly as possible.

If you'd like to support me, follow me on twitch:
https://www.twitch.tv/fosterbarnes

or if you're feeling generous drop a donation:
https://coff.ee/fosterbarnes

## Notes about v5.0.3 and v5.0.4
1. If you encounter this error when launching Yap Bot:
   
  ![errorScreenshot](https://i.postimg.cc/NMtTnkmt/Virtual-Box-VM-f5dcj-WRF3-O.png)

- Download and install [vc_redist.x86.exe](https://aka.ms/vs/17/release/vc_redist.x86.exe) then restart the app

2. Starting in v5.0.3, Windows may incorrectly flag the installer as a virus. You may need to allow your browser to download it. If Windows blocks it from running, click the windows security notification that pops up, then allow it to run. More info on setting exclusions in Windows Security if needed: https://www.elevenforum.com/t/add-or-remove-exclusions-for-microsoft-defender-antivirus-in-windows-11.8797/

	This is a known problem with the app, but not much can be done about it. Fixing this false flag would mean paying hundreds of dollars a year for code-signing. The component that's most likely triggering antivirus is [traymond-tcp](https://github.com/fosterbarnes/traymond-tcp). Yap Bot is built on the [egui](https://github.com/emilk/egui) library, which does not have the ability to natively minimize windows to the system tray. To be able to add this feature, I had to fork the original build of [traymond](https://github.com/fcFn/traymond) and edit it to be able to communicate with Yap Bot. The original traymond waits for a set keyboard combination from the user, then minimizes the selected window to tray when those keys are pressed. Because of this keyboard monitoring, some anti-viruses interpret this as malicious and attempt to block it. In it's current re-worked state, we don't even use the key-combo function, and just use it to receive commands from Yap Bot, then minimize Yap Bot to tray, but this original code remains in the project.

	All of that being said, always exercise caution when running unknown apps from github. This app and traymond-tcp are completely open source, so feel free to go through the code and build for yourself if you're worried about anything malicious.

<details>
  <summary><h2>Changelog</h2></summary>

  <h3>v5.0.0</h3>

  <h4>YapBotInstaller</h4>
  <ul>
    <li>Improve UI experience for entering configuration details</li>
    <li>Auto-add bot account when authenticating oauth & access token</li>
    <li>Check & warn when using main account for yap bot</li>
    <li>Added automatic update checking</li>
  </ul>

  <h4>MarkovChainBot</h4>
  <ul>
    <li>Correctly pass cooldown value as an integer</li>
    <li>Display cooldown one time per cooldown period if bot is activated during this period</li>
    <li>Fix bug where bot would sometimes say "I haven't extracted &quot;&quot; from chat yet." when generate message is sent</li>
    <li>Implements manual trigger functionality for Yap Bot via file and TCP methods to enable external or GUI-based activation</li>
    <li>Various fixes</li>
  </ul>

  <h4>TwitchYapBot</h4>
  <ul>
    <li>Added Settings menu</li>
    <li>Improved UI</li>
    <li>Added fun stuff</li>
    <li>Automatic version checking and installation</li>
  </ul>

  <h4>Added Updater</h4>
  <ul>
    <li>Automatically updates TwitchYapBot</li>
  </ul>

  <h3>v5.0.1</h3>

  <h4>TwitchYapBot</h4>
  <ul>
    <li>Code refactoring</li>
    <li>Display settings icon in the settings window instead of egui default</li>
    <li>Changes settings cog button rendering method because it looked crusty</li>
    <li>Fixed bug where clicking the cancel button in settings would make the settings window black and fail to close it</li>
    <li>Improved debug console output</li>
    <li>App now creates a log file and prints debug output. Saves the most recent 10 run logs and deletes the oldest one if necessary</li>
    <li>Made the output section collapsable. Fade in/out animation when hiding/showing</li>
    <li>Added sound effects. Changed file name formatting for sounds</li>
    <li>Dynamically add sound effects when compiling instead of using a static array</li>
    <li>Properly check for the current version being newer than the newest public release</li>
    <li>The repeated logic for centering the window is now stored in <code>center_window.rs</code> and called when needed elsewhere</li>
  </ul>

  <h3>v5.0.2</h3>

  <h4>MarkovChainBot</h4>
  <ul>
    <li>Allow users to set a generation timer with a randomized number of seconds</li>
    <li>Set generation timer minimum to 5 seconds instead of 30</li>
  </ul>

  <h4>TwitchYapBot</h4>
  <ul>
    <li>Added setting in UI for randomized generation timers</li>
    <li>Timers now have an on/off checkbox instead of having the user enter a negative number to disable it</li>
    <li>Added a tip that explains what these settings do when hovering over the checkbox</li>
    <li>Added tips on hover for all bot settings</li>
    <li>Allows user to generate a new bot token from the GUI. Not having this button was an oversight</li>
    <li>Change "Authentication" -&gt; "Access Token" in settings to be more clear</li>
    <li>Don't show "#" prefix in channel name settings</li>
  </ul>

  <h3>v5.0.3</h3>

  <h4>TwitchYapBot</h4>
  <ul>
    <li>Added option to start app minimized to tray</li>
    <li>Added button to minimize to tray</li>
    <li>Added <code>traymond-tcp.exe</code> as a resource. This binary is based on <a href="https://github.com/fcFn/traymond">traymond</a>. It has been forked to have TCP functionality</li>
    <li>Update installer to install <code>traymond-tcp.exe</code></li>
    <li>Update updater to install and/or update <code>traymond-tcp.exe</code></li>
    <li>Added option to automatically close when OBS or Streamlabs OBS close</li>
    <li>Added "first launch" setting and popup for announcing new features after updating</li>
  </ul>

  <h3>v5.0.4</h3>

  <h4>YapBotInstaller</h4>
  <ul>
    <li>Fix issue where desktop and start menu shortcuts are sometimes not created</li>
    <li>Improved installer logic and reliability</li>
    <li>Checks for installed versions of x86 &amp; x64 builds of Microsoft Visual C++ Redistributable. Installs them if necessary</li>
  </ul>

  <h4>YapBotUpdater</h4>
  <ul>
    <li>Checks for installed versions of x86 &amp; x64 builds of Microsoft Visual C++ Redistributable. Installs them if necessary</li>
  </ul>

  <h4>TwitchYapBot</h4>
  <ul>
    <li>OBS monitoring PowerShell process runs hidden with no window shown</li>
  </ul>

  <h3>v5.1.0</h3>

  <h4>YapBotTray</h4>
  <ul>
    <li>A new standalone binary that will run the Python bot and live in the Windows system tray with no GUI. You are still able to open the GUI from the tray app, however, and vice versa</li>
    <li>This app still respects the "close when OBS closes" setting, as well as the "start app to tray" setting</li>
  </ul>

  <h4>TwitchYapBot</h4>
  <ul>
    <li>Improved efficiency for OBS exit monitoring. Completely re-wrote the logic for OBS monitoring to be more simple and efficient</li>
    <li>Completely re-wrote the logic and method for minimizing to tray. <code>traymond-tcp</code> is no longer used. During testing I found that minimizing egui apps to the system tray will use WAY more CPU resources than it should (up to around 10% CPU usage). This had nothing to do with the code itself, but is a limitation of egui itself. We now use a custom binary called <code>YapBotTray.exe</code> that is able to run the bot itself, and launch the main GUI application. This new tray app only uses a few MB of RAM, and ~0% of the CPU.</li>
    <li>Added window state debug logging (minimized, un-minimized etc.)</li>
  </ul>

  <h4>YapBotUpdater</h4>
  <ul>
    <li>Removes the need to download and install <code>traymond-tcp.exe</code></li>
    <li>Installs the new tray app: <code>YapBotTray.exe</code></li>
    <li>Installs the icon used by the tray app: <code>yap_icon_purple.ico</code></li>
  </ul>

  <h4>YapBotInstaller</h4>
  <ul>
    <li>Removes the need to install <code>traymond-tcp.exe</code></li>
    <li>Installs the new tray app: <code>YapBotTray.exe</code></li>
    <li>Installs the icon used by the tray app: <code>yap_icon_purple.ico</code></li>
  </ul>
</details>
