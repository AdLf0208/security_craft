# Working on this Horizon native mod

This is a **flat SDK package** (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs`
(`impl mod_api::Hooks`) + `src/support.rs` (the game glue) + `src/runtime.rs` (the pure rules and
their tests) + `assets/` + `data/`. Horizon retired the `.hnmod` container in the mod-API merge
(#16941), so there is no `compatibility.json`, `mixins/<id>/`, `horizon-mixin.toml`, plan/injection
schema or `rust-toolchain.toml` here, and none of them should be recreated.

> **Build slots (v2):** every build/test goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a slot is
> free, so **never poll and never background a build**; a timeout exits **75**. It does **not**
> `cd` into the port — wrap the command (`bash -c 'cd ports/<id> && …'`) and never pass
> `--target-dir`. Read `../_shared/build-slots-usage.md` before running anything.

```sh
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step test \
    -- bash -c 'cd ports/securitycraft && CARGO_INCREMENTAL=0 cargo test --offline'
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step lint \
    -- bash -c 'cd ports/securitycraft && cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step build \
    -- bash -c 'cd ports/securitycraft && CARGO_INCREMENTAL=0 cargo horizon-mod build'
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step check \
    -- bash -c 'cd ports/securitycraft && cargo horizon-mod check securitycraft.hmod &&
        cargo horizon-mod fits securitycraft.hmod --build nightly'
```

- `cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never packed).
  `build` packs the `.hmod`; it does **not** compile against the game — that is
  `horizon-launcher native-build . --instance <instance> --trust-native-code`.
- **`src/runtime.rs` carries the retired bundle's rules**, plus this port's review-fix rules
  (minimum PIN length, `Action::TooShort`, the retry `Throttle`); its sha256 is
  `8f78aba553247a123cac17888e593b81c52c84c230d0f95de010c0b97ecb03f3` (the pre-review carried file was
  `1b2744a909c36c98d4c78dc09a28a060709c1fa17543ffed571f5c384fb1756a`): the behaviour rules live there,
  `src/support.rs` and `src/lib.rs` only convert SDK types to theirs. New rule → a test in
  `runtime.rs`; new glue → a private fn and a test in `support.rs`.
- Only the API the SDK publishes: `cargo horizon-mod hooks <text>` finds the exact method; never
  guess a name, and record an API gap in `notes.md` instead of faking one.
- The delivered package is `securitycraft.hmod` **in the port root**, never in `target/`;
  `LICENSE` and `LICENSE-NOTICE.md` travel inside it.
- `sha2` is this package's one registry dependency (the carried rules module hashes the code);
  its `Cargo.lock` therefore ships inside the package — unlike the lock-free content-only ports.
- Keep the port separate from Horizon and horizon-mixins; never modify the pin checkouts, never
  commit binaries, exported sources or Minecraft assets, and never put Mojang files in the tree.
- `python3 ports/tools/check-skills.py --notes ports/securitycraft` must stay green, and
  `notes.md` records every claim with its command and output.
