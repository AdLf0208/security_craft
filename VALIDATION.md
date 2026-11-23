# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.5**, validated on
Linux with rustc 1.97.0 on 2026-11-23 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. Every step ran in a
`ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; the evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| Red (the SHA-256 primitive stubbed, `sha2` removed) | 7 failed / 36 passed, incl. the existing digest vector (`r9-red.log`) |
| `cargo test --offline` | 43 passed, 0 failed (`r9-test.log`) |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`r9-clippy.log`) |
| `cargo horizon-mod build` twice + `cmp` | byte-identical (`r9-build-1.log`, `-2.log`) |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.5 (native-mod, … for 26.3)` (`r9-check.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`r9-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same artifact SHA (`r9-fits-other.log`) |
| Clean build of the extracted archive | `Compiling securitycraft v0.2.5`, `built`, exit 0 (`r9-clean-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1 (`r9-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0` (`securitycraft-e2e/e2e-launcher.log`) |
| Control (same scenario, unmodified Nightly) | `EXIT:1` (the mod's ids are unknown) (`e2e-control.log`) |
| `SELFTEST=1` (door), `owner`, `relaunch` | `EXIT:1` each (`e2e-selftest.log`, `e2e-selftest-owner.log`, `e2e-selftest-relaunch.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | green |

The delivered package is `securitycraft.hmod` in this directory; its size and SHA-256 are in
`notes.md` ("R9 — the hash is the port's own code") and in the build log. It declares **no
dependency but the SDK's `horizon-mod-api`** (registry rule R9) and therefore **packs no
`Cargo.lock`** (R10 accepts a lock only as the previous release's): the keypad's salted SHA-256
is this port's own code in `src/runtime.rs`. The "declare a registry dependency and pack its
lock" policy the 0.2.3/0.2.4 packages used is dead for publishing — `#1800` was refused for
exactly that.

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world) and drives placement, the drawn keypad panel, wrong/right codes, the harming module's
recipe, install, damage, drop, a **level-side `/setblock` removal** that must keep the module,
the differential keypad, the **owner gate on setting a code** — a stranger's click opens no set
screen, a set screen that outlived an owner change stores nothing, the owner's set works, and a
stranger's `/securitycraft code` is refused while the owner's code survives — and a
**quit-and-relaunch** leg in which the code stored before the restart still opens the same
keypad (so the digest is recomputed from the saved salt, not held in the session). The stranger
is the harness-only `/securitycraft owner <pos> stranger`. It needs a launcher-built game
carrying the package: `ports/tools/launcher-game.sh build securitycraft --port
ports/securitycraft`, then `eval "$(ports/tools/launcher-game.sh env securitycraft)"`. No player
data is read; every run uses a fresh scratch `HORIZON_HOME`, `XDG_DATA_HOME` and
`XDG_CONFIG_HOME`, and the world lives under the run's `--game-dir`.

The slice-level red/green record, the coverage table and the remaining gaps are in `notes.md`.
