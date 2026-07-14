# Polycore

Polycore is a native endless free-for-all arena shooter built with Rust and Bevy. Farm geometric shapes, specialize a tank across two evolution tiers, and take first place from five persistent adaptive opponents.

The match does not stop for evolution choices or player death. A death resets that tank's build and life score, while the arena and match-wide K/D continue. `Retry` returns only the player to the live match; `Continue` starts a fresh match.

## Playable Slice

- Five mechanically identical AI tanks with distinct playstyles, adaptive builds, threat assessment, defensive retaliation, retreats, and limited crown challenges
- A unique-leader crown, live streak records, crown markers, and an off-screen leader indicator
- Persistent profile records, achievements, settings, and twelve cosmetic palettes
- Eight upgrade paths and eight level-5 evolutions, each with two branch-specific level-15 successors
- Shared FFA damage rules for projectiles, penetration, body contact, splash, marks, shields, armor, spawn protection, and kill rewards
- Tier-colored farm shapes, owner-colored projectiles, stable bot identities, and an evolution-aware live leaderboard
- Mouse-wheel zoom, explicit Escape pause, pooled particles, directional damage indicators, and configurable camera shake

## Controls

| Input | Action |
| --- | --- |
| `W` `A` `S` `D` | Move |
| Mouse | Aim |
| Left mouse button | Shoot |
| Mouse wheel | Zoom |
| `1` through `8` | Spend an upgrade point |
| `Escape` | Pause or resume |

Evolution choices, palettes, and settings use the mouse. Evolution and death overlays do not pause the world.

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

Tagged GitHub releases produce Linux and Windows archives containing the executable, assets, default profile configuration, README, license, and credits.

## Profile Data

`profile.json` is stored under the platform configuration directory:

- Linux: `$XDG_CONFIG_HOME/polycore` or `~/.config/polycore`
- Windows: `%APPDATA%\Polycore`

Writes are atomic. A malformed profile is backed up as `profile.corrupt.json` before defaults are loaded. Combat-affecting progression never persists between lives.

## Status

This is an early single-player playable slice. Networking, additional evolution tiers, account services, and competitive matchmaking are outside the current scope. Balance and presentation remain subject to playtest iteration.

## License

Polycore is source available under the [Polycore Source Available License v1.0](LICENSE.md).
