# Validation

The current package is the flat SDK package (mod API 0.2) at version **0.2.6**, validated on
Linux with rustc 1.97.0 on 2026-11-24 against Horizon
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. Compile/test/format/check steps ran through
**forwarded** `ports/tools/build-slot.sh` leases on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`); the launcher and game legs stayed local
because the remote box has no launcher or prepared resources and the e2e is `--offscreen`.
`CARGO_INCREMENTAL=0` throughout; **no `BUILD_LOCAL=1`**. Evidence is under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Result |
|---|---|
| Red (the allowlist rule stubbed) | **43 passed / 2 failed**, incl. "a listed player is allowed without typing a code" (`slice5-red.log`) |
| `cargo test --offline` | 47 passed, 0 failed (`slice5-test.log`, `sdk-test.log`) |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --offline -- -D warnings` | Pass, 0 warnings (`slice5-lint.log`) |
| Two `cargo horizon-mod build`s + `cmp` | byte-identical (`slice5-build.log`), `rm -f Cargo.lock` before each |
| `cargo horizon-mod check securitycraft.hmod` | exit 0, `0.2.6 (native-mod, … for 26.3)` (`slice5-check-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice5-fits.log`) |
| `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0, same artifact SHA (`slice5-fits.log`) |
| Clean launcher build | `Compiling securitycraft (1 of 1)`; no registry crates; `native-0a875da-59c49fbba148` (`slice5-launcher-build.log`) |
| `python3 ports/tools/verify-sdk-package.py securitycraft` | 1/1, incl. `package matches the tree`, SDK-only deps, no packed lock (`slice5-verify-sdk-package.log`) |
| In-game scenario (`tools/e2e.sh`, launcher-built game) | PASS + `EXIT:0`, incl. phase 14 allowlist (`securitycraft-e2e/e2e-launcher.log`) |
| Control (full scenario, unmodified Nightly) | `EXIT:1` at phase 1 (`e2e-control.log`) |
| Ids-unknown control (`CONTROL=1`) | PASS + `EXIT:0`; both module ids unknown (`e2e-ids-unknown.log`) |
| `SELFTEST=1`, `harming`, `nonloot`, `owner`, `relaunch` | `EXIT:1` each (`e2e-selftest*.log`) |
| `SELFTEST=allow` (new allowlist branch) | `EXIT:1` at phase 14 (`e2e-selftest-allow.log`) |
| `check-e2e-logs.py securitycraft-e2e --strict` | `logs=9 ok=9`, exit 0 (`slice5-check-e2e-logs.log`) |
| `check-recipe-types.py securitycraft` | `securitycraft: ok` |

The delivered package is `securitycraft.hmod` **64,114 B**, SHA-256
`8df73c3810f2ad10d1c473b5583eaf4752b6cff047081614b554fd3ca65d80a1`, **52 entries**, version
**0.2.6**. It declares no dependency but the SDK's `horizon-mod-api` (registry rule R9) and packs no
`Cargo.lock`. The keypad's salted SHA-256 and the new allowlist rules remain this port's own code in
`src/runtime.rs`, byte-identical inside the package.

`tools/e2e.sh` launches a real game with `horizon-ctl` (offscreen renderer, creative, a scratch
world) and now also proves the allowlist module: its recipe is crafted in game (output id, count and
an emptied grid), the module is installed on a coded keypad, `/securitycraft allow <pos> self` puts
the acting player on the list, and a click with nothing typed powers the keypad and opens the
adjacent door **without a code screen**. Swapping the list to `stranger` makes the same click open
the code screen with the door closed; `SELFTEST=allow` inverts the first assertion. The
phase-11b owner flake recorded in `notes.md` is pre-existing and did not recur in this wave.

The slice-level red/green record, the upstream rules as read, the coverage table and the remaining
gaps are in `notes.md`.
