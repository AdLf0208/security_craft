# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.7**, validated on
Linux with rustc 1.95.0 on 2026-11-24 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. Compile/test/format/check steps ran through
**forwarded** `ports/tools/build-slot.sh` leases on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`, relative `--port ports/securitycraft`);
the launcher and game legs stayed local because the remote box has no launcher or prepared
resources and the e2e is `--offscreen`. `CARGO_INCREMENTAL=0` throughout; **no `BUILD_LOCAL=1`**.
Evidence is under `../_shared/evidence/securitycraft/` and
`../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| Red (the `is_denied` rule stubbed) | **48 passed / 1 failed**, "a listed player is denied, upstream's ListModuleData.isPlayerOnList" (`slice6-red.log`) |
| `cargo test --offline` | **51 passed**, 0 failed (`sdk-test.log`, regenerated for 0.2.7) |
| `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice6-lint.log`) |
| Two `cargo horizon-mod build`s + `cmp` | byte-identical (`slice6-build.log`), `rm -f Cargo.lock` before each |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.7 (native-mod, … for 26.3)`, sha `7f1f7243…48f7c` (`slice6-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice6-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same artefact SHA (`slice6-check-fits.log`) |
| Clean launcher build | stale `native-*` tag carrying securitycraft removed first; `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-91db09b0e4bc` (`slice6-launcher-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1, incl. `package matches the tree`, SDK-only deps, no packed lock (`slice6-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0`, incl. phase 14 allowlist, phase 16 denylist and phase 15 relaunch (`securitycraft-e2e/e2e-launcher.log`) |
| Control (full scenario, unmodified Nightly) | `EXIT:1` at phase 1 (`e2e-control.log`) |
| Ids-unknown control (`CONTROL=1`) | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| `SELFTEST=1`, `harming`, `nonloot`, `owner`, `relaunch`, `allow` | `EXIT:1` each (`e2e-selftest*.log`) |
| `SELFTEST=deny` (new deny branch) | `EXIT:1` at phase 16 (`e2e-selftest-deny.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=11 ok=11`, exit 0 (`slice6-check-e2e-logs.log`) |
| `check-recipe-types.py securitycraft` | `securitycraft: ok` |

The delivered package is `securitycraft.hmod` **67,128 B**, SHA-256
`7f1f7243129083bee719a20c14b2c232d2ed1807c4d9b539b2c28bb6aee48f7c`, **57 entries**, version
**0.2.7**. It declares no dependency but the SDK's `horizon-mod-api` (registry rule R9) and packs
no `Cargo.lock`. The keypad's salted SHA-256, the allowlist rules and the new denylist rule remain
this port's own code in `src/runtime.rs`, byte-identical inside the package.

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world). Phase 16 now also proves the denylist module: its recipe is crafted in game (output id,
count and an emptied grid), the module is installed on a coded keypad with an adjacent door,
`/securitycraft deny <pos> self` puts the acting player on the list, and a click opens **nothing**
(`screen=playing`), powers nothing and leaves the door closed. Swapping the list to `stranger`
makes the identical click open the code screen with the door still closed; `SELFTEST=deny`
inverts the first assertion. The pre-existing phase-11b module-stash flake recorded in `notes.md`
did not recur in the PASS leg; any leg that stops there is retried transparently rather than
called green.

The slice-level red/green record, the upstream rules as read, the coverage table and the remaining
gaps are in `notes.md`.
