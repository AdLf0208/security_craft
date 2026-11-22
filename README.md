# SecurityCraft (keypad, reinforced door, harming module) — Horizon native mod

The SecurityCraft port for **Horizon**, as a flat SDK package: mod API **0.2**, built
against Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

- **`horizon-mod.toml`** — the only manifest and the only identity
  (`id = "securitycraft"`, `version = "0.2.4"`, `side = "both"`, `[native]` pins).
- **`src/runtime.rs`** — the pure rules (code shape, salted digest, verdict, entry,
  mode, signal countdown, who may set a code or install a module, and the wrong-code
  damage) and their tests.
- **`src/support.rs`** — the game glue: the block entity's saved data (code, salt,
  owner, modules), the screen's key rectangles and drawing, the menu buttons, the
  redstone, the owner gate on setting a code, the module install, the wrong-code
  damage, the module drop and the harness commands.
- **`src/lib.rs`** — the one `impl mod_api::Hooks`, the keypad's Parts and the key
  panel item's Part (which records the owner).
- **`assets/`, `data/`** — the carried content: the `securitycraft:keypad` block and
  its `keypad_item`, the `securitycraft:iron_door_reinforced` block, the harming
  module (`securitycraft:harming_module`, its recipe and its
  `securitycraft:incorrect_passcode` damage type), the jar's blockstates, models,
  textures, loot, recipes, tag and lang, plus the keypad's content menu type.
- **`securitycraft.hmod`** — the delivered package (built by
  `cargo horizon-mod build`, copied out of `target/`). It **packs `Cargo.lock`**: this is the one
  package in the campaign that declares a registry crate (`sha2`), and the launcher's clean build
  (`native-build`) refuses a package that declares one without its lock.
- **`FINDINGS.md`** — the upstream behaviour read; **`notes.md`** — scope, hook map,
  deviations, verification and gaps.

## Build and check

The campaign's `build-slot.sh` is a **lease**; the commands inside are the SDK tool's
own. Builds that need `cargo-horizon-mod` run locally (`BUILD_LOCAL=1`):

```sh
ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft --step test -- \
    bash -c 'cd ports/securitycraft && CARGO_INCREMENTAL=0 cargo test --offline'
ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft --step lint -- \
    bash -c 'cd ports/securitycraft && cargo fmt --check &&
        cargo clippy --all-targets --offline -- -D warnings'
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft \
    --step build -- bash -c 'cd ports/securitycraft && CARGO_INCREMENTAL=0 cargo horizon-mod build'
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft \
    --step check -- bash -c 'cd ports/securitycraft &&
        cargo horizon-mod check securitycraft.hmod &&
        cargo horizon-mod fits securitycraft.hmod --build 0a875da'
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never
packed). `build` **packs** the `.hmod`; it does not compile against the game — that is
the launcher's `native-build`.

## In game

`ports/tools/launcher-game.sh build securitycraft --port ports/securitycraft` compiles
this package into the pinned Nightly and exports it; `tools/e2e.sh` then drives a real
game with `horizon-ctl` (place, screen keys, wrong/right codes, module install, health,
recipe, break drops, a level-side `/setblock` removal that must keep the module, and the
owner gate on setting a code — proved by moving the owner to a stranger with the
harness-only `/securitycraft owner`). `SELFTEST=1`, `SELFTEST=harming` and
`SELFTEST=owner` prove the assertions can fail, and `GAME=$CONTROL_GAME` runs the same
scenario on an unmodified game, which fails at the first mod id. The evidence is under
`ports/_shared/evidence/securitycraft-e2e/`.

The mod is **local-only** until the maintainer approves publishing
(ARCHITECTURE.md §8); the upstream jar and its assets are MIT.
