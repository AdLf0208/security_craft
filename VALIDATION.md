# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.8**, validated on
Linux with rustc 1.95.0 on 2026-11-24 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. All Rust steps ran through **forwarded**
`ports/tools/build-slot.sh` leases on `AdLf-PC` (`build-slot: forwarding … to the remote runner`,
relative `--port ports/securitycraft`); the launcher build and game legs stayed local because the
remote box has no launcher, no prepared 26.3 resources and no renderer for this `--offscreen` e2e.
`CARGO_INCREMENTAL=0` throughout; **no `BUILD_LOCAL=1`**. Evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| Red (the smart-module gate stubbed) | **52 passed / 2 failed**, "five wrong codes must not rate-limit a keypad without the smart module" and "the same budget is ignored without the module" (`slice7-red.log`) |
| `cargo test --offline` | **54 passed**, 0 failed (`slice7-test.log`, `sdk-test.log` regenerated for 0.2.8) |
| `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice7-lint.log`) |
| Two `cargo horizon-mod build`s + `cmp` | byte-identical (`slice7-build.log`), `rm -f Cargo.lock` before each |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.8 (native-mod, … for 26.3)`, sha `b9388415…4051` (`slice7-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice7-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same artifact SHA (`slice7-check-fits.log`) |
| Clean launcher build | stale 0.2.7 `native-*` tag moved aside first; `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-6abd9a7c8302` (`slice7-launcher-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1, incl. `package matches the tree`, SDK-only deps, no packed lock, 54 tests (`slice7-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0`, incl. phase 3b (no module → no cooldown) and phase 17 (smart module → `status=6`, unpowered, door closed) (`securitycraft-e2e/e2e-launcher.log`) |
| Control (full scenario, unmodified Nightly) | `EXIT:1` at phase 1 (`e2e-control.log`) |
| Ids-unknown control (`CONTROL=1`) | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| `SELFTEST=smart` | `EXIT:1` at phase 3b, "expected a no-module keypad to be rate-limited" (`e2e-selftest-smart.log`) |
| `SELFTEST=1`, `harming`, `nonloot`, `owner`, `relaunch`, `allow`, `deny` | `EXIT:1` at each own assertion (`e2e-selftest*.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=13 ok=13`, exit 0 (`slice7-check-e2e-logs.log`) |
| `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

The delivered package is `securitycraft.hmod` **69,773 B**, SHA-256
`b93884158f168af3e41b84dd700ce47befcdacecf49ea0e5a759bd744c504051`, **61 entries**, version
**0.2.8**. It declares no dependency but the SDK's `horizon-mod-api` (registry rule R9) and packs
no `Cargo.lock`. The keypad's salted SHA-256, the smart-module retry gate, the allowlist/denylist
rules and the deny-before-allow order remain this port's own code in `src/runtime.rs`, byte-identical
inside the package.

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world). Phase 17 now also proves the smart module: its recipe is crafted in game (output id, count
and an emptied grid), the module is installed on a second coded keypad with an adjacent door, five
wrong codes are typed and the right one must then read `status=6`, `powered=false` and door
`open=false`. The same five wrong codes on the first keypad, which has no module, still let the
right code open it (`status=3`, `powered=true`, door `open=true`); `SELFTEST=smart` inverts that
no-module assertion. The allowlist, denylist, owner and relaunch legs stay green in the same run.

One transient: the first PASS-leg attempt stopped at phase 17's keypad placement after the smart
recipe and install succeeded (`FAIL: the smart keypad did not place`, `EXIT:1`). No source or package
byte changed before the retry, which added only a diagnostic echo (extra ctl round-trips); the same
placement then answered `changed:true` on air and the whole leg passed. The excerpt is kept in
`securitycraft-e2e/e2e-smart-first-attempt-flake.log`; it is recorded, not called green. The older
phase-11b module-stash flake did not recur this wave.

The slice-level red/green record, the upstream rules as read, the coverage table and the remaining
gaps are in `notes.md`.
