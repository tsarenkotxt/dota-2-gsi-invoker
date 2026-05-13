# Dota 2 GSI Invoker Overlay

Rust overlay for Invoker in Dota 2. It reads Game State Integration data and displays all 10
Invoker skills in a compact two-row overlay, with Dota-like cooldown and mana availability states.

The overlay is designed for low overhead: it uses localhost GSI data, embedded skill icons,
and a small always-on-top window.
> Use Dota 2 Borderless Window mode. Exclusive fullscreen can hide normal desktop overlays.

![Invoker overlay](docs/screenshot.png)

[Download the latest release](../../releases/latest)

## Dota 2 Setup

Copy `gamestate_integration_dota_2_gsi_invoker.cfg` into Dota's `gamestate_integration` directory:

| OS      | Path                                                                                                                                         |
|---------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Windows | `C:\Program Files (x86)\Steam\steamapps\common\dota 2 beta\game\dota\cfg\gamestate_integration\gamestate_integration_dota_2_gsi_invoker.cfg` |

> Create the `gamestate_integration` subdirectory if it does not exist.

## App Config

The config file is optional. When present, the app reads `dota_2_gsi_invoker_config.json`
from the same directory where it is started. When missing, built-in defaults are used:

| Key               | Description                                                                    |
|-------------------|--------------------------------------------------------------------------------|
| `gsi_port`        | Local port used by the app and GSI cfg. Must match the cfg URI port            |
| `debug_gsi`       | Prints raw GSI payloads when `true`, keep `false` during normal play           |
| `show_footer_row` | Adds an empty third row below the skill grid when `true`                       |
| `overlay_x`       | Overlay X position. Use `overlay_y = -1` for automatic placement               |
| `overlay_y`       | Overlay Y position. Use `overlay_x = -1` for automatic placement               |
| `skill_mana_cost` | Mana cost per skill, used because Dota GSI does not reliably expose it         |
| `skill_order`     | Display order. First five skills are the top row, next five are the bottom row |

> The current `overlay_x` and `overlay_y` values are configured for a 4K resolution.

## Dev Requirements

| Requirement | Notes                            |
|-------------|----------------------------------|
| Rust        | `1.95.0` or higher               |
| Make        | Used to run the project commands |

## Make Commands

Run these from a shell in the project root:

| Command                | Description                                                              |
|------------------------|--------------------------------------------------------------------------|
| `make init`            | Install Clippy and the Windows Rust target, then fetch dependencies      |
| `make run`             | Run the app on the current platform                                      |
| `make test`            | Run formatting, unit tests, and Clippy                                   |
| `make release-windows` | Build a 64-bit Windows release and package files into `release/windows/` |

## Is It Allowed?

This app uses Dota 2 Game State Integration data and renders a normal external overlay window.
It does not inject into Dota 2, read game memory, modify game files, or hook the renderer.

It is still your responsibility to use it in a way that fits the current Dota 2, Steam,
and tournament rules that apply to you.
