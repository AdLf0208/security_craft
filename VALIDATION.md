# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.4**, validated on
Linux with rustc 1.97.0 on 2026-11-22 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. Every step ran in a
`ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; the evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| Red (the owner rule stubbed) | 1 failed / 41 passed, "a stranger sets nothing" (`slice4-red.log`) |
| `cargo test --offline` | 42 passed, 0 failed (`slice4-test.log`) |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice4-clippy.log`) |
| `cargo horizon-mod build` twice + `cmp` | byte-identical (`slice4-build-1.log`, `-2.log`) |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.4 (native-mod, … for 26.3)` (`slice4-check.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice4-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same artifact SHA (`slice4-fits-other.log`) |
| Clean build of the extracted archive | `Compiling securitycraft v0.2.4`, `native-0a875da-e141cb80db51 built`, exit 0 (`slice4-clean-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1 (`slice4-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0` (`securitycraft-e2e/e2e-launcher.log`) |
| Control (same scenario, unmodified Nightly) | `EXIT:1` (the mod's ids are unknown) (`e2e-control.log`) |
| `SELFTEST=1` (door), `SELFTEST=owner` (the refused set inverted) | `EXIT:1` each (`e2e-selftest.log`, `e2e-selftest-owner.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=7 ok=7`, exit 0 |

The delivered package is `securitycraft.hmod` in this directory: **59,524 B**, SHA-256
`eb42fd50fa0b81da38a083b2dd0c6b25847133d4aa8705469f15992b07679b91`, **48 entries**. It **packs
`Cargo.lock`** (49 packages) because it declares a registry crate (`sha2`), and the launcher's
clean build path refuses a package that declares one without a lock. That is a deliberate
package-scoped exception to the campaign's usual lock-free package (ARCHITECTURE.md §5), recorded
in `notes.md` and in `Cargo.toml` beside the dependency; `verify-sdk-package.py` accepts it
(`packed Cargo.lock (declares a dependency)`).

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world) and drives placement, the drawn keypad panel, wrong/right codes, the harming module's
recipe, install, damage, drop, a **level-side `/setblock` removal** that must keep the module,
the differential keypad, and the **owner gate on setting a code** — a stranger's click opens no set
screen, a set screen that outlived an owner change stores nothing, the owner's set works, and a
stranger's `/securitycraft code` is refused while the owner's code survives. The stranger is the
harness-only `/securitycraft owner <pos> stranger`. It needs a launcher-built game carrying the
package: `ports/tools/launcher-game.sh build securitycraft --port ports/securitycraft`, then
`eval "$(ports/tools/launcher-game.sh env securitycraft)"`. No player data is read; every run uses
a fresh scratch `HORIZON_HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, and the world lives under
the run's `--game-dir`.

The slice-level red/green record, the coverage table and the remaining gaps are in `notes.md`
("Slice 4 — the owner gate on setting a code").
