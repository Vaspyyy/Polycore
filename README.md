# Polycore

Polycore is a native endless free-for-all arena shooter built with Rust and Bevy. Farm geometric shapes, specialize a tank across three evolution tiers, and take first place from five persistent adaptive opponents.

The match does not stop for evolution choices or player death. A death resets that tank's build and life score, while the arena and match-wide K/D continue. `Retry` returns only the player to the live match; `New Match` starts a fresh match.

## Playable Slice

- Five mechanically identical AI tanks with distinct playstyles, adaptive builds, threat assessment, defensive retaliation, retreats, and limited crown challenges
- A unique-leader crown, live streak records, crown markers, and an off-screen leader indicator
- Persistent profile records, achievements, settings, and sixteen cosmetic palettes
- Eight upgrade paths, eight level-5 evolutions, sixteen level-15 branches, and sixteen level-30 capstones with bespoke active abilities
- Recurring high-tier shape hotspots and a permanent full-arena minimap for the player, leader, and active zone
- Shared FFA damage rules for projectiles, penetration, body contact, splash, marks, shields, armor, spawn protection, and kill rewards
- Tier-colored farm shapes, owner-colored projectiles, stable bot identities, and an evolution-aware live leaderboard
- Mouse-wheel zoom, explicit Escape pause, pooled particles, directional damage indicators, configurable camera shake, and a manual low-power mode

## Controls

| Input | Action |
| --- | --- |
| `W` `A` `S` `D` | Move |
| Mouse | Aim |
| Left mouse button | Shoot |
| Right mouse button | Use the level-30 active ability |
| Mouse wheel | Zoom |
| `1` through `8` | Spend an upgrade point |
| `F3` | Toggle the session performance overlay |
| `Escape` | Pause or resume |

Evolution choices, palettes, and settings use the mouse. Evolution and death overlays do not pause the world. The pause menu includes a persistent low-power toggle; it disables MSAA and the zoom vignette and reduces cosmetic burst particles without changing gameplay entities or simulation behavior.

## Build

Polycore requires a current Rust toolchain with Rust 2024 edition support.

```bash
cargo run
```

Validation commands:

```bash
cargo fmt -- --check
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
cargo test --release -- --ignored stress_harness
```

Tagged GitHub releases produce Linux and Windows archives containing the executable, default profile configuration, README, license, and credits. Game visuals are generated procedurally, so no external asset bundle is required.

## Profile Data

`profile.json` is stored under the platform configuration directory:

- Linux: `$XDG_CONFIG_HOME/polycore` or `~/.config/polycore`
- Windows: `%APPDATA%\Polycore`

Writes are atomic. A malformed profile is backed up as `profile.corrupt.json` before defaults are loaded. Combat-affecting progression never persists between lives.

Profile progress remains in memory during active play so filesystem stalls cannot interrupt combat. Dirty progress is saved on pause, death, returning to the menu, and clean application exit. Forcibly terminating the process during an active life can therefore lose progress accumulated since the last safe-state save.

The F3 overlay shows recent average and p95 FPS, the worst recent frame, fixed simulation steps, and live entity counters. Frames slower than 50 ms are sampled in memory and flushed only in safe states to `performance.log` beside `profile.json`; the log rotates at 512 KiB to `performance.previous.log`. It contains technical counters, phase, resolution, fullscreen state, and low-power state only—never the player name and never a network upload.

## Status

This is an early single-player playable slice. Networking, account services, and competitive matchmaking are outside the current scope. Balance and presentation remain subject to playtest iteration.

## License

Polycore is source available under the [Polycore Source Available License v1.0](LICENSE.md).
