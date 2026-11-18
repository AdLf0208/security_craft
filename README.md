# SecurityCraft (keypad + reinforced door) — Horizon native mod

Slice 1 of the SecurityCraft port for **Horizon**, as a flat SDK package: mod
API **0.2**, built against Horizon
`0b14f8acfd97fa2c750e42795249dcc981178ca4` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

- **`horizon-mod.toml`** — the only manifest and the only identity
  (`id = "securitycraft"`, `version = "0.2.0"`, `side = "both"`, `[native]` pins).
- **`src/runtime.rs`** — the pure keypad rules (code shape, salted digest,
  verdict, entry, mode, signal countdown) and their 28 tests; **byte-identical**
  to the retired `.hnmod` bundle's `runtime.rs` (SHA-256 `1b2744a9…56a`).
- **`src/support.rs`** — the game glue: the block entity's saved data, the
  screen's key rectangles and drawing, the menu buttons, the redstone and the
  harness commands.
- **`src/lib.rs`** — the one `impl mod_api::Hooks` and the keypad's Parts.
- **`assets/`, `data/`** — the carried content: the `securitycraft:keypad` block
  and its `keypad_item`, the `securitycraft:iron_door_reinforced` block, the
  jar's blockstates, models, textures, loot, recipe, tag and lang, plus the
  keypad's content menu type (one slot, six data slots).
- **`securitycraft.hmod`** — the delivered package (built by
  `cargo horizon-mod build`, copied out of `target/`).
- **`FINDINGS.md`** — the upstream behaviour read; **`notes.md`** — scope, the
  hook map, deviations, verification and gaps.

## Build and check

The campaign's `build-slot.sh` is used only as a **lease**; its staged/mixin
steps do not apply to an SDK package, so the commands inside are the SDK tool's
own:

```sh
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step test -- \
    bash -c 'cd ports/securitycraft && CARGO_INCREMENTAL=0 cargo test --offline'
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step lint -- \
    bash -c 'cd ports/securitycraft && cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step build -- \
    bash -c 'cd ports/securitycraft && CARGO_INCREMENTAL=0 cargo horizon-mod build'
ports/tools/build-slot.sh run --job sdk-sc --port ports/securitycraft --step check -- \
    bash -c 'cd ports/securitycraft && cargo horizon-mod check securitycraft.hmod &&
        cargo horizon-mod fits securitycraft.hmod --build nightly'
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never
packed). `build` **packs** the `.hmod`; it does not compile against the game —
that is the launcher's `native-build`, which is not available here.

`tools/e2e.sh` is the retired bundle's in-game scenario, kept as the behaviour
oracle: it needs a launcher-built game carrying this package and has not been
re-run since the migration (see `notes.md`).

The mod is **local-only** until the maintainer approves publishing
(ARCHITECTURE.md §8); the upstream jar and its assets are MIT.
