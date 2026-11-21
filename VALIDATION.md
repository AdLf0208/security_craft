# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.2**, validated on
Linux with rustc 1.97.0 on 2026-11-21 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. Every step ran in a
`ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; the evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings |
| `cargo test --offline` | 40 passed, 0 failed |
| `cargo horizon-mod build` twice (`rm -f Cargo.lock` before each) | byte-identical |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.2 (native-mod, … for 26.3)` |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1 |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0` |
| Control (same scenario, unmodified Nightly) | `EXIT:1` (the mod's ids are unknown) |
| `SELFTEST=1` (door), `SELFTEST=harming` (module) | `EXIT:1` each |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=5 ok=5`, exit 0 |

The delivered package is `securitycraft.hmod` in this directory: **52,759 B**, SHA-256
`3497bfcca16344bb6d6f4dd03b48ae6e125aabfcbb1d7d87023a0ab445681d11`, **47 entries**, with **no
`Cargo.lock`** packed. `Cargo.lock` stays in the tree for the launcher's `native-build` (pinned to
`libc 0.2.190` so it matches the Nightly's graph); it is not part of the artifact.

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world) and drives the placement, the drawn keypad panel, wrong/right codes, the harming module's
recipe, install, damage, drop and the differential. It needs a launcher-built game carrying the
package: `ports/tools/launcher-game.sh build securitycraft --port ports/securitycraft`, then
`eval "$(ports/tools/launcher-game.sh env securitycraft)"`. No player data is read; every run uses
a fresh scratch `HORIZON_HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, and the world lives under
the run's `--game-dir`.

The slice-level red/green record, the coverage table and the remaining gaps are in `notes.md`
("Slice 3 — the harming module").
