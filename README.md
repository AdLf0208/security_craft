# Security Craft — a Horizon port

**Security Craft** is a Minecraft mod. Source code for the Minecraft mod "SecurityCraft". This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Geforce132/SecurityCraft](https://github.com/Geforce132/SecurityCraft) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Security Craft ([source](https://github.com/Geforce132/SecurityCraft)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `securitycraft.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- Keypads, reinforced doors and the rest of SecurityCraft's block protection, ported to Hori

## Not yet ported

- **No in-game run at `0b14f8a`** — no launcher and no game build at this pin (trap 9). Everything
- **No `BlockMetadata` from a `&dyn LevelHost`** (trap 3): `powered` is read from the serialised
- **A registry dependency needs `horizon-launcher native-lock .`** on the building machine
- **The slice's own gaps are unchanged** from the retired bundle: owner/allowlist/denylist, the

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `securitycraft.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Security Craft was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Geforce132/SecurityCraft

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run securitycraft` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/securitycraft/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
