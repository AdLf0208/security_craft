# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.11** (the passcode
furnace slice), validated on Linux with rustc 1.95.0 on 2026-11-24 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. All Rust steps ran through **forwarded**
`ports/tools/build-slot.sh` leases on `AdLf-PC` (`build-slot: forwarding … to the remote runner`,
relative `--port ports/securitycraft`); the launcher build and game legs stayed local because the
remote box has no launcher, no prepared 26.3 resources and no renderer for this `--offscreen` e2e.
`CARGO_INCREMENTAL=0` throughout; **no `BUILD_LOCAL=1`**. Evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

## Slice 11 — the passcode furnace (0.2.11)

| Check | Result |
|---|---|
| Red (`furnace_access`/`furnace_tick` stubbed) | 57 passed / 2 failed, both new assertions (`slice10-red.log`) |
| `cargo test --offline` | **59 passed**, 0 failed (`slice10-test.log`, `sdk-test.log`) |
| `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice10-lint.log`) |
| Two `cargo horizon-mod build`s + `cmp` | byte-identical, `rm -f Cargo.lock` before each (`slice10-build*.log`) |
| `cargo horizon-mod check securitycraft.hmod` | `0.2.11 (native-mod, … for 26.3)`, EXIT 0 (`slice10-check-fits.log`) |
| `fits --build 0a875da` / `--build 0b14f8a` | `made for` / `fits`, delivered SHA in both (`slice10-check-fits.log`) |
| `verify-sdk-package.py securitycraft` | 1/1, `package matches the tree`, 59 tests, no lock (`slice10-verify-sdk-package.log`) |
| Launcher build | `Compiling securitycraft (1 of 1)` (`slice10-launcher-build.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | **PASS + `EXIT:0`**, all 20 phases incl. phase 20 (`securitycraft-e2e/e2e-launcher.log`) |
| Control (unmodified Nightly) | `EXIT:1` at phase 1 (`e2e-control.log`) |
| Ids-unknown control (`CONTROL=1`) | PASS + `EXIT:0`, ids unknown (`e2e-ids-unknown.log`) |
| `SELFTEST=furnace` | `EXIT:1` at phase 20 (`e2e-selftest-furnace.log`) |
| Prior SELFTESTs (`1`, `harming`, `nonloot`, `owner`, `relaunch`, `allow`, `deny`, `smart`, `disguise`, `container`) | `EXIT:1` at each own assertion (`e2e-selftest*.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=17 ok=17`, exit 0 (`slice10-check-e2e-logs.log`) |
| `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

The delivered package is `securitycraft.hmod` **95,921 B**, SHA-256
`b3271e8884e04fc0a878eaf90a171bb7d49f8a6c34f36c94993448879a7e6f67`, **95 entries**, version
**0.2.11**, SDK-only dependency set (R9) and no packed `Cargo.lock`. The launcher build printed
`Compiling securitycraft (1 of 1)` after the stale 0.2.10 tag was moved aside. `sdk-test.log` was
regenerated on the final 0.2.11 tree (59 tests) before the last `verify-sdk-package.py` run, which
reports `package matches the tree`.

The furnace phase proves the carried `keypad_furnace`: an unprotected vanilla furnace opens without
a code; a wrong `copen` leaves the passcode furnace closed with no slots; the right code opens its
three-slot block-backed menu; raw iron and coal cook into one iron ingot with the input consumed
and `lit=true`; and the code and output survive a relaunch. `SELFTEST=furnace` inverts the
wrong-code assertion and fails exactly there.

## Slice 9 — the passcode barrel (0.2.10)

| Check | Result |
|---|---|
| Red (`container_access` stubbed) | 55 passed / 1 failed, `left: WrongCode right: Open` (`slice9-red.log`) |
| `cargo test --offline` | **57 passed**, 0 failed (`slice9-test.log`, `sdk-test.log`) |
| `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice9-fmt.log`, `slice9-clippy.log`) |
| Two `cargo horizon-mod build`s + `cmp` | byte-identical (`slice9-build.log`), no `Cargo.lock` |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.10 (native-mod, … for 26.3)`, sha `2829d08f…41841` (`slice9-check-fits.log`) |
| `fits --build 0a875da` / `--build 0b14f8a` | `made for` / `fits`, same SHA (`slice9-check-fits.log`) |
| Launcher build | `Compiling securitycraft (1 of 1)`, `native-0a875da-fa8e073dbac3 built` (`slice9-launcher-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1, `package matches the tree`, 57 tests, no lock (`slice9-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | **PASS + `EXIT:0`**, all 19 phases incl. phase 19 (`securitycraft-e2e/e2e-launcher.log`) |
| Control (unmodified Nightly) | `EXIT:1` at phase 1 (`e2e-control.log`) |
| Ids-unknown control (`CONTROL=1`) | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| `SELFTEST=container` | `EXIT:1`, "expected a wrong code to leave the barrel's contents unreadable" (`e2e-selftest-container.log`) |
| Prior SELFTESTs (`1`, `harming`, `nonloot`, `owner`, `relaunch`, `allow`, `deny`, `smart`, `disguise`) | `EXIT:1` at each own assertion (`e2e-selftest*.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=16 ok=16`, exit 0 (`slice9-check-e2e-logs.log`) |
| `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

The delivered package is `securitycraft.hmod` **83,579 B**, SHA-256
`2829d08f2d21a71db2e47381215d0d50f3fbdaba0c18ee29b632fa20e1d41841`, **78
entries**, version **0.2.10**, SDK-only dependency set (R9) and no packed
`Cargo.lock`. Phase 19 proves the carried `keypad_barrel`: the owner sets a code,
a wrong code leaves it closed with no container slots, the right code opens its
27-slot menu, eight ingots are read back in slot 300 and survive a relaunch, an
unknown id is refused, and an unprotected vanilla barrel opens with no code.
`SELFTEST=container` inverts the wrong-code assertion and fails exactly there.
The pure `container_access` rule and the in-tree SHA-256 are unchanged by the
other slices' assertions; deny-before-allow, the smart gate and the disguise
render stay green in the same PASS leg.

### Earlier verification (0.2.9 and previous)

| Check | Result |
|---|---|
| Red (the `disguise_tint` rule referenced before definition) | `cannot find function 'disguise_tint' in this scope`, `EXIT:101` (`slice8-red.log`) |
| `cargo test --offline` | **55 passed**, 0 failed (`slice8-lint.log`, `sdk-test.log` regenerated for 0.2.9) |
| `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice8-lint.log`) |
| Two `cargo horizon-mod build`s + `cmp` | byte-identical (`slice8-build.log`, `slice8-build2.log`), `rm -f Cargo.lock` before each |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.9 (native-mod, … for 26.3)`, sha `b9f614ef…0b88` (`slice8-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice8-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same artifact SHA (`slice8-check-fits.log`) |
| Clean launcher build | stale 0.2.8 `native-*` tag moved aside first; `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-36c1a04c654d` (`slice8-launcher-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1, incl. `package matches the tree`, SDK-only deps, no packed lock, 55 tests (`slice8-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0`, all 18 phases incl. phase 18's screenshot pixel difference (`securitycraft-e2e/e2e-launcher.log`) |
| Control (full scenario, unmodified Nightly) | `EXIT:1` at phase 1 (`e2e-control.log`) |
| Ids-unknown control (`CONTROL=1`) | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| `SELFTEST=disguise` | `EXIT:1` at phase 18, "expected the disguised keypad NOT to be green-dominant" (`e2e-selftest-disguise.log`) |
| Prior SELFTESTs (`1`, `harming`, `nonloot`, `owner`, `relaunch`, `allow`, `deny`, `smart`) | `EXIT:1` at each own assertion (`e2e-selftest*.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=15 ok=15`, exit 0 (`slice8-check-e2e-logs.log`) |
| `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

The delivered package is `securitycraft.hmod` **73,976 B**, SHA-256
`b9f614efd830c11a0e1b3416736ec841ea37855cb84cc7fa208b410b880`, **65 entries**, version
**0.2.9**. It declares no dependency but the SDK's `horizon-mod-api` (registry rule R9) and packs
no `Cargo.lock`. The keypad's salted SHA-256, the smart-module retry gate, the allowlist/denylist
rules and the deny-before-allow order remain this port's own code in `src/runtime.rs`, byte-identical
inside the package.

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world). Phase 18 proves the disguise module: its recipe is crafted in game (output id, count and an
emptied grid), the module is installed on a fresh keypad through the owner gate, and the **same**
keypad is photographed from one camera before and after. The plain face reads
`PIXEL 176 176 176`; the disguised face reads `PIXEL 6 11 2` (green-dominant by ratio and 509
darker in total). `SELFTEST=disguise` inverts the green-dominance assertion and fails exactly there.
All 17 prior phases and every prior inverted leg stay green in the same build.

One pre-existing e2e harness trap was diagnosed this slice: temporarily widening `info()` from
`ctl messages 8` to `ctl messages 30` consistently broke phase 2's second key sequence (the code was
never stored, `status=5`), so the change was reverted and the full leg then passed. The two render
diagnostics that shaped the implementation — `draw.cube`'s texel units and this engine build's
ignored `tint` argument, resolved by drawing a lime-concrete block by key through
`host.block_key(...)` + `draw.block(...)` — are recorded in `notes.md` with their frames.

The slice-level red/green record, the upstream rules as read, the coverage table and the remaining
gaps are in `notes.md`.
