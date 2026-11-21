# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.3**, validated on
Linux with rustc 1.97.0 on 2026-11-21 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. Every step ran in a
`ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; the evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass (`review-1544-fmt.log`) |
| `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`review-1544-clippy.log`) |
| `cargo test --offline` | 41 passed, 0 failed (`review-1544-test.log`) |
| `cargo horizon-mod build` twice + `cmp` | byte-identical (`review-1544-build-1.log`, `-2.log`) |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.3 (native-mod, … for 26.3)` (`review-1544-check.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0, sha `978a2365…` (`review-1544-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same sha (`review-1544-fits-other.log`) |
| Launcher clean build of the extracted archive | `native-0a875da-6744658cf70e built`, exit 0 (`review-1544-lock-clean-build.log`) |
| Same extraction without `Cargo.lock` (counterfactual) | `error: …Cargo.lock: No such file or directory (os error 2)`, exit 1 (`review-1544-red-lock-final.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1 (`review-1544-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0` (`securitycraft-e2e/e2e-launcher.log`) |
| Control (same scenario, unmodified Nightly) | `EXIT:1` (the mod's ids are unknown) (`e2e-launcher-control.log`) |
| `SELFTEST=1` (door), `SELFTEST=harming` (module) | `EXIT:1` each (`e2e-selftest.log`, `e2e-selftest-harming.log`) |
| `SELFTEST=nonloot` (the phase 11b assertion inverted) | `EXIT:1`, "expected the module to vanish with a level-side removal" (`e2e-selftest-nonloot.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=6 ok=6`, exit 0 |

The delivered package is `securitycraft.hmod` in this directory: **57,881 B**, SHA-256
`978a23652d2580b7e0486ff0d7f819c20330593fff3ab140edde5ef1f19e7112`, **48 entries**. It **packs
`Cargo.lock`** (49 packages, sha256 `8d448753f648…b296315`) because it declares a registry crate
(`sha2`), and the launcher's clean build path refuses a package that declares one without a lock.
That is a deliberate package-scoped exception to the campaign's usual lock-free package
(ARCHITECTURE.md §5), recorded in `notes.md` ("Review fixes (PR #1544)") and in `Cargo.toml` beside
the dependency; `verify-sdk-package.py` accepts it (`packed Cargo.lock (declares a dependency)`).
The clean-build receipt for this exact archive is
[`../_shared/evidence/securitycraft/review-1544-lock-receipt.md`](../_shared/evidence/securitycraft/review-1544-lock-receipt.md).

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world) and drives the placement, the drawn keypad panel, wrong/right codes, the harming module's
recipe, install, damage, drop, a **level-side `/setblock` removal** that must keep the module as a
dropped item, and the differential. It needs a launcher-built game carrying the package:
`ports/tools/launcher-game.sh build securitycraft --port ports/securitycraft`, then
`eval "$(ports/tools/launcher-game.sh env securitycraft)"`. No player data is read; every run uses
a fresh scratch `HORIZON_HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, and the world lives under
the run's `--game-dir`.

The slice-level red/green record, the coverage table and the remaining gaps are in `notes.md`
("Slice 3 — the harming module" and "Review fixes (2026-11-21, PR #1544) — 0.2.3").
