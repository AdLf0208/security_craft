# SecurityCraft (keypad, reinforced door, passcode barrel/furnace/smoker/trapdoor/door and the five modules) — Horizon native mod

The SecurityCraft port for **Horizon**, as a flat SDK package: mod API **0.2**, built
against Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

- **`horizon-mod.toml`** — the only manifest and the only identity
  (`id = "securitycraft"`, `version = "0.2.15"`, `side = "both"`, `[native]` pins).
- **`src/runtime.rs`** — the pure rules (code shape, salted digest, verdict, entry,
  mode, signal countdown, who may set a code or install a module, the allowlist's and
  denylist's listed players, the wrong-code damage, the smart-module retry gate and the
  passcode container's owner-and-code access rule) and
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
  cooldown, the passcode containers' code-set/unlock/readout and one-shot open gate, the
  passcode furnace's smelting tick and the smoker's smoking tick (fuel, cook time and `lit`,
  against the game's own recipe table), the passcode trapdoor's code-gated `open` flip and its
  closing countdown, the passcode door's two-half `open` flip and countdown, the module drop and
  the harness commands.
- **`src/lib.rs`** — the one `impl mod_api::Hooks`, the keypad's, the passcode barrel's,
  the passcode furnace's and the passcode smoker's Parts, and their items' Parts (which record
  the owner).
- **`assets/`, `data/`** — the carried content: the `securitycraft:keypad` block and
  its `keypad_item`, the `securitycraft:iron_door_reinforced` block, the harming
  module (`securitycraft:harming_module`, its recipe and its
  `securitycraft:incorrect_passcode` damage type), the allowlist module
  (`securitycraft:allowlist_module`, carried as the jar's `whitelist_module`), the
  denylist module (`securitycraft:denylist_module`, carried as the jar's
  `blacklist_module`, III/IPI/ISI with an ink sac), the jar's
  blockstates, models,
  textures, loot, recipes, tag and lang, plus the smart module (securitycraft:smart_module,
  carried as the jar's smart_module, III/IPI/ISI with an ender pearl), the keypad's
  content menu type, the passcode barrel (`securitycraft:keypad_barrel`, a carried
  `minecraft:barrel` with 27 slots shown as the vanilla 9×3 menu), the passcode furnace
  (`securitycraft:keypad_furnace`, a carried `minecraft:furnace` with the jar's blockstate,
  models and five textures, trimmed to the 26.3 `facing`/`lit` states), and the passcode
  smoker (`securitycraft:keypad_smoker`, a carried `minecraft:smoker` with the jar's
  `keypad_smoker_*` models and window texture, cooked against `minecraft:smoking`), and the
  passcode trapdoor (`securitycraft:keypad_trapdoor`, the jar's 16-state blockstate, three static
  block models and texture on a `minecraft:iron_trapdoor` template; `open` flips only for the
  owner's right code and closes on the scheduled tick), and the passcode door
  (`securitycraft:keypad_door`, the jar's 32-state blockstate and eight static models on a
  `minecraft:iron_door` template; the owner's right code flips `open` on both halves and the
  scheduled tick closes them both), and the universal block modifier — the remover and level-1
  reinforcer tools (`securitycraft:universal_block_remover`,
  `securitycraft:universal_block_reinforcer_lvl1`) that convert a carried reinforced cube
  (`reinforced_stone`, `reinforced_iron_block`, `reinforced_oak_planks`) to and from its vanilla
  counterpart, carrying the block's owner/code state across the swap.
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
one without it), the passcode barrel, furnace and smoker (wrong code → no menu or slots, right code → a
block-backed container; the furnace cooks raw iron and coal to one iron ingot and the smoker cooks
food against the smoking table, with the input consumed and `lit=true`; code and contents survive a
relaunch, against unprotected vanilla containers), and a quit-and-relaunch leg where the code stored
before the restart still opens the keypad).
`SELFTEST=1`, `SELFTEST=harming`, `SELFTEST=nonloot`, `SELFTEST=owner`, `SELFTEST=allow`,
`SELFTEST=deny`, `SELFTEST=smart`, `SELFTEST=disguise`, `SELFTEST=container`,
`SELFTEST=furnace`, `SELFTEST=smoker`, `SELFTEST=keypad-trapdoor` and
`SELFTEST=keypad-door-full` prove the
assertions can fail, and
`GAME=$CONTROL_GAME` runs the same scenario on an unmodified game, which fails at the
first mod id. The evidence is under `ports/_shared/evidence/securitycraft-e2e/`.

The mod is **local-only** until the maintainer approves publishing
(ARCHITECTURE.md §8); the upstream jar and its assets are MIT.
