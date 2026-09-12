<h1 align="center">Needlelight</h1>

<p align="center">
A modern desktop mod manager for Hollow Knight and Hollow Knight: Silksong, built with Tauri and Rust.
</p>

<div align="center">
<table>
<tr>
<td align="center">
  <img src="screenshots/main.png" alt="Needlelight Main Interface" width="300"/>
  <br />
  <b>Main Interface</b>
</td>
<td align="center">
  <img src="screenshots/api.png" alt="Needlelight API Interface" width="300"/>
  <br />
  <b>API</b>
</td>
<td align="center">
  <img src="screenshots/settings.png" alt="Needlelight Settings" width="300"/>
  <br />
  <b>Settings</b>
</td>
</tr>
</table>
</div>

<div align="center">

![build](https://github.com/Aarav2709/Needlelight/actions/workflows/build.yml/badge.svg)
[![GitHub all releases](https://img.shields.io/github/downloads/Aarav2709/Needlelight/total)](https://github.com/Aarav2709/Needlelight/releases)

</div>

## About
- Needlelight is a desktop mod manager for Hollow Knight and Hollow Knight: Silksong.
- The project has been rebuilt from the ground up around a Tauri and Rust architecture, with a native Rust backend handling game files, mod installation, profiles, and other system level operations.
- The goal is to provide a simple way to discover, install, manage, and configure mods without having to manually deal with game files.

## Project Status and Attribution

- Needlelight is not associated with Lumafly.
- The current codebase is a fresh rewrite built around Tauri and Rust. It does not use the legacy Lumafly implementation as runtime code.
- The project was inspired by the work that came before it.


### Reference and Investigation Sources

The following projects and repositories were referred to during Needlelight development, debugging, and implementation research. They are credited here as references and sources of ideas or implementation patterns. This does not mean their code is included wholesale in Needlelight.

#### Scarab
- Repository: https://github.com/fifty-six/Scarab
- Referred to for Hollow Knight API installation, API state handling, vanilla/modded `Assembly-CSharp.dll` swapping, mod installation, and extraction behavior.
- Scarab is licensed under GPL-3.0.

#### Cogfly
- Repository: https://github.com/Nix-main/Cogfly
- Referred to for Hollow Knight: Silksong game path handling and Silksong mod management implementation patterns.
> The developer made several dismissive remarks about the project, including referring to AI assisted bug fixing as "slop", questioning the project's motivations, and making negative comments about the launcher. I found this unnecessary, particularly given that I am 15 and this is an independent project. I plan to publish a blog post with more details about the situation for transparency.

#### Hollow Knight Modding API
- Repository: https://github.com/hk-modding/api
- Referred to as the upstream Hollow Knight Modding API source.
- Needlelight uses API release information supplied through the modlinks ecosystem rather than treating arbitrary repository commits as releases.

#### Hollow Knight ModLinks
- Repository: https://github.com/hk-modding/modlinks
- Referred to as the source of official mod and API distribution metadata used by the launcher.

#### ItemChanger Unity 6 Port
- Repository: https://github.com/bibobonking/ItemChanger-port
- Referred to during investigation of Hollow Knight 1.5.12620 / Unity 6 compatibility and the community API work around that game version.
- This repository is an external community project and is not a Needlelight dependency.

#### Modrinth / Theseus
- Repository: https://github.com/modrinth/code
- The Needlelight frontend is built using parts of Modrinth's open source Theseus launcher project.
- This includes the Vue.js frontend, `@modrinth/ui` component library, and `@modrinth/assets` icon set.
- Modrinth's code is licensed under the GNU General Public License v3.
- Huge thanks to the Modrinth team for making their work available as open source.
- I personally e-mailed the team as well, for permission, they allowed me to do so.
> I have used the base, and iterated it accordingly, so it doesn't comes off as a rip-off of modrinth. Several features were stripped off, which were not required for my project.

#### Lucide
- Website: https://lucide.dev/
- Referred to for iconography through the Modrinth assets ecosystem.

## Usage
- Download the latest version from the [Releases](https://github.com/Aarav2709/Needlelight/releases) page and launch Needlelight.
- Hollow Knight is selected by default.
- To manage Silksong, open Settings and select Silksong under Game. Needlelight keeps the selected game profile between launches.
- Once a game is configured, you can browse the available mods, install the ones you want, and manage your installed mods directly from Needlelight.
- Installed mods can be enabled or disabled from the application.
- Needlelight can also be used offline to manage mods that are already installed.

## Features
### Game Profiles
- Needlelight supports both Hollow Knight and Hollow Knight: Silksong.
- Each game has its own profile and game directory, so switching between games does not require manually changing paths every time.

### Mod Management
- Browse available mods through the official modlinks catalog.
- Install and manage mods directly from Needlelight without manually moving files around.
- You can also manually install mods that are not available through the catalog.

### Modding API
- Needlelight can install and manage the required Modding API for supported games.
- The API can also be toggled to switch between a modded and vanilla setup.
- For Silksong, Needlelight handles the BepInEx installation required for mod support.

## Silksong Support
- Needlelight supports Hollow Knight: Silksong as a separate game profile.
- When Silksong is selected, Needlelight uses the configured Silksong installation directory instead of the Hollow Knight directory.
- If a Silksong installation has not been configured yet, Needlelight will ask you to select the game folder.
- Needlelight also handles the BepInEx setup required for Silksong when the Modding API is installed.

## Custom Modlinks
- Needlelight supports custom ModLinks catalogs.
- Open Settings and enable `Use Custom Modlinks`, then provide the URL of a ModLinks.xml file.
- This can be useful for community maintained catalogs or personal forks of the official modlinks repository.
- Custom catalogs are saved separately for each game profile.
- To return to the official catalog, disable `Use Custom Modlinks` in Settings.

## Installation Diagnostics
- Needlelight records installation related information in its log file.
- If a mod installation fails or a mod appears to install correctly but does not load, the installation log can be useful when reporting the issue.

The log can be found at:
### Windows
```text
%APPDATA%\HKModInstaller\Needlelight-install.log
```

### macOS
```text
~/Library/Application Support/HKModInstaller/Needlelight-install.log
```

### Linux
```text
~/.config/HKModInstaller/Needlelight-install.log
```
If you have set `XDG_CONFIG_HOME`, the log will instead be located under:
```text
$XDG_CONFIG_HOME/HKModInstaller/Needlelight-install.log
```

## Backend Configuration
- The desktop application supports overriding the backend catalog endpoints through environment variables.
- These settings are primarily useful for development and testing different catalog sources.

The available variables are:
```text
NEEDLELIGHT_HK_MODLINKS_URLS
NEEDLELIGHT_HK_APILINKS_URLS
NEEDLELIGHT_SS_MODLINKS_URLS
NEEDLELIGHT_SS_APILINKS_URLS
```

Multiple URLs can be provided as a comma separated list.
For example:
```bash
NEEDLELIGHT_HK_MODLINKS_URLS="https://your.backend/ModLinks.xml"
NEEDLELIGHT_HK_APILINKS_URLS="https://your.backend/ApiLinks.xml"
```

## Development
- Needlelight is built using Tauri, Rust, and Vue.
- Install the project dependencies and use the project's development commands to start the desktop application.
- The Rust backend is located in the Tauri application and handles native functionality such as filesystem access, game detection, mod installation, and configuration.

## Contributing
- Contributions are welcome.
- If you find a bug, have an idea, or want to suggest an improvement, open an issue on GitHub.
- If you want to contribute code, feel free to open a pull request.
