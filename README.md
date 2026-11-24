# SecurityCraft (keypad, reinforced door, harming, allowlist, denylist and smart modules) — Horizon native mod

The SecurityCraft port for **Horizon**, as a flat SDK package: mod API **0.2**, built
against Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

- **`horizon-mod.toml`** — the only manifest and the only identity
  (`id = "securitycraft"`, `version = "0.2.8"`, `side = "both"`, `[native]` pins).
- **`src/runtime.rs`** — the pure rules (code shape, salted digest, verdict, entry,
  mode, signal countdown, who may set a code or install a module, the allowlist's and
  denylist's listed players, the wrong-code damage and the smart-module retry gate) and
  their tests. It also carries the
  port's **own SHA-256**, because the
  registry refuses a package that declares any dependency but the SDK
  (`tools/native-subset/src/manifests.rs`, R9) — the digest crate cannot travel with a
  native `.hmod`.
- **`src/support.rs`** — the game glue: the block entity's saved data (code, salt,
  owner, modules), the screen's key rectangles and drawing, the menu buttons, the
  redstone, the owner gate on setting a code, the module install, the wrong-code
  damage, the denylist's "denied → no interaction" branch (before allow), the
  allowlist's "allowed → open without a code" branch, the smart module's gate on the retry
  cooldown, the module drop and the harness commands.
- **`src/lib.rs`** — the one `impl mod_api::Hooks`, the keypad's Parts and the key
  panel item's Part (which records the owner).
- **`assets/`, `data/`** — the carried content: the `securitycraft:keypad` block and
  its `keypad_item`, the `securitycraft:iron_door_reinforced` block, the harming
  module (`securitycraft:harming_module`, its recipe and its
  `securitycraft:incorrect_passcode` damage type), the allowlist module
  (`securitycraft:allowlist_module`, carried as the jar's `whitelist_module`), the
  denylist module (`securitycraft:denylist_module`, carried as the jar's
  `blacklist_module`, III/IPI/ISI with an ink sac), the jar's
  blockstates, models,
  textures, loot, recipes, tag and lang, plus the smart module (securitycraft:smart_module,
  carried as the jar's smart_module, III/IPI/ISI with an ender pearl), and the keypad's
  content menu type.
- **`securitycraft.hmod`** — the delivered package (built by
  `cargo horizon-mod build`, copied out of `target/`). Its only dependency is the SDK's
  `horizon-mod-api`, so it **packs no `Cargo.lock`**: R10 accepts a lock only as the
  previous release's, and an SDK-only package needs none.
- **`FINDINGS.md`** — the upstream behaviour read; **`notes.md`** — scope, hook map,
  deviations, verification and gaps.

## Build and check

The campaign's `build-slot.sh` is a **lease**; the commands inside are the SDK tool's
own. Compile steps now forward to the repaired second box automatically — do **not**
set `BUILD_LOCAL=1`. The relative `--port ports/securitycraft` form is required for
forwarding, and the remote starts each command inside the mirrored port directory
(no `cd` wrapper):

```sh
ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft --step test -- \
    bash -c 'CARGO_INCREMENTAL=0 cargo test --offline'
ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft --step lint -- \
    bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'
ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft --step build -- \
    bash -c 'rm -f Cargo.lock && CARGO_INCREMENTAL=0 cargo horizon-mod build'
ports/tools/build-slot.sh run --job W1-securitycraft --port ports/securitycraft --step check -- \
    bash -c 'cargo horizon-mod check securitycraft.hmod &&
        cargo horizon-mod fits securitycraft.hmod --build 0a875da'
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never
packed). `build` **packs** the `.hmod`; it does not compile against the game — that is
the launcher's `native-build`.

## In game

`ports/tools/launcher-game.sh build securitycraft --port ports/securitycraft` compiles
this package into the pinned Nightly and exports it; `tools/e2e.sh` then drives a real
game with `horizon-ctl` (place, screen keys, wrong/right codes, module install, health,
recipe, break drops, a level-side `/setblock` removal that must keep the module, the
owner gate on setting a code — proved by moving the owner to a stranger with the
harness-only `/securitycraft owner` — the allowlist module's crafted recipe, install and
"allowed → open without a code" branch (with a stranger differential), the denylist
module's crafted recipe and its "denied → no interaction" branch, the smart module's
gate on the retry cooldown (five wrong codes lock a keypad with the module but not
one without it), and a quit-and-relaunch leg where the code stored before the restart still opens the keypad).
`SELFTEST=1`, `SELFTEST=harming`, `SELFTEST=nonloot`, `SELFTEST=owner`, `SELFTEST=allow`,
`SELFTEST=deny`, `SELFTEST=smart` and `SELFTEST=relaunch` prove the assertions can fail, and
`GAME=$CONTROL_GAME` runs the same scenario on an unmodified game, which fails at the
first mod id. The evidence is under `ports/_shared/evidence/securitycraft-e2e/`.

The mod is **local-only** until the maintainer approves publishing
(ARCHITECTURE.md §8); the upstream jar and its assets are MIT.
