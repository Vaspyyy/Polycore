# Polycore

Polycore is a native single-player geometric arena shooter built around fast tank combat, shape farming, stat upgrades, and tank evolutions.

The current prototype drops the player into a free-for-all arena with five adaptive enemy bots. Every tank can farm shapes for XP, level up, choose upgrades, evolve into a specialized build, and fight for the top position on the live leaderboard.

## Current Features

- Free-for-all combat against five persistent enemy bots
- Distinct bot playstyles with adaptive upgrades, threat assessment, defensive behavior, opportunistic attacks, and low-health retreats
- XP-bearing polygon shapes that spawn around active tanks
- Eight upgrade paths: health regeneration, max health, body damage, bullet speed, bullet penetration, bullet damage, reload, and movement speed
- Eight level-five evolutions with different weapons and stat tradeoffs: Gunner, Cannon, Twin Barrel, Sniper, Ram Core, Sprayer, Guard, and Flanker
- Projectile, body, shape, knockback, health, regeneration, and respawn systems
- Live leaderboard showing score, player name, kills, deaths, and each tank's current evolution
- Persistent match state when retrying after death, with a full reset available from the death screen
- Mouse-wheel camera zoom with soft and hard zoom-out limits

## Controls

| Input | Action |
| --- | --- |
| `W` `A` `S` `D` | Move |
| Mouse | Aim |
| Left mouse button | Shoot |
| Mouse wheel | Zoom in or out |
| `1` through `8` | Spend an upgrade point on the matching stat |

Evolution choices and menu actions are selected with the mouse.

## Running The Game

Polycore requires a Rust toolchain with Rust 2024 edition support.

```bash
cargo run
```

For a compile-only development check:

```bash
cargo check
```

## Project Status

Polycore is an early playable prototype. The current scope is single-player arena combat and AI-driven progression. Multiplayer networking, additional evolution tiers, content expansion, balance work, and release packaging are not implemented yet.

The codebase and gameplay systems are actively changing, so saved behavior and balance should not be considered stable.

## Tech

- Rust 2024 edition
- Bevy 0.19
- WGPU through Bevy's renderer

## License

Polycore is source available under the [Polycore Source Available License v1.0](LICENSE.md). Non-commercial use, modification, and redistribution are permitted under strong copyleft terms. Derivative works must remain source available under the same license, and commercial use requires a separate license.
