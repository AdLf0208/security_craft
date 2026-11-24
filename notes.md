# SecurityCraft → Horizon (slice 1: keypad + reinforced door)

> **Format (2026-11-18): the port is now a flat SDK `.hmod`.** Horizon retired `.hnmod` in the
> mod-API merge (#16941, launcher ≥ 0.2.70); the bundle this file described was migrated to
> `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` (`impl mod_api::Hooks`) + `src/support.rs` +
> `src/runtime.rs` + `assets/`/`data/` at Horizon **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** /
> horizon-mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API `0.2`, port version
> **0.2.0**. Read **`## Migration to the SDK format`** at the end of this file first: it is the
> current state. Everything above it is the history of the retired bundle (pin
> `1d63e0a5`/`5a64af93`, `.hnmod`, `mixins/`, the staging ladder) and is kept as the record, not
> as instructions — none of `tools/horizon.py`, `compatibility.json`, `mixins/<id>/`, `content/`
> or the `.hnmod` exists any more (they are under
> `../_shared/evidence/securitycraft/retired-hnmod-0.1.0/`).

Ported from **SecurityCraft 1.10.2.1** (`[1.21.1] SecurityCraft v1.10.2.1.jar`,
SHA-1 `8760079e80039f982f1a92d3ee9d60a2bcab3251`), **MIT** (the jar's
`neoforge.mods.toml`) → **licence rank 0**. Upstream
`Geforce132/SecurityCraft` branch `1.21.1`, commit
`c4d35b0c8bf85c2a94c37a1f8ce37ef5ce1a6aa8`. Provenance:
`ports/_shared/carry-work/securitycraft/SOURCE.md`; the behaviour read:
[`FINDINGS.md`](FINDINGS.md). The upstream source was read for behaviour only;
this is a clean-room Rust rewrite. Local-only until publishing is approved
(ARCHITECTURE.md §8).

**Why this mod now:** a **core** content mod (no required ATM10 content parent)
at licence rank 0, and the next small self-contained mechanics job: two blocks,
one block entity, one screen and one redstone signal. The full mod is a
campaign-scale set of systems (cameras, mines, lasers, scanners, disguises), so
it is delivered in slices and this is slice 1.

## Class + tier and the approach

- **Class:** `content`; **tier:** `deep` (`ports/_shared/source-map/
  atm10-sources.csv`, mirroring `ports/ledger.csv`).
- **Approach:** content + agent port. Per `horizon/docs/modpacks.md` ("Tiers",
  "Carry-over: data and assets", "Code: one agent per mod"):
  - the **data half** is carried from the jar — the keypad's blockstate,
    models and two textures, the reinforced door's blockstate, eight models and
    two textures, the two loot tables, the key panel recipe, the
    `mineable/pickaxe` tag and the lang entries (plus the 26.3 `items/` model
    files the 1.21.1 jar does not have);
  - the **code half** is the block entity (`code`/`salt`/`mode`/`entry`/
    `signal`), the keypad screen and menu buttons, the redstone answer and the
    countdown, because the tier's `deep` label in this pack is about the mod's
    cameras/lasers/disguises; this slice is the **logic + machines** shape the
    two blocks actually need (block entity, menu, redstone), and the deep half
    (cameras, mines) stays a gap.
- The reinforced door is the carried `minecraft:iron_door` template: vanilla's
  door behaviour (redstone-only, double block, sounds) with the mod's ids,
  textures, hardness and loot. No door code is needed for this slice.

## Slice boundary

**In:** the `securitycraft:keypad` block and its `securitycraft:keypad_item`
item; the code screen (digits 0–9, backspace, submit) and its menu buttons; the
salted code storage and the comparison; the `powered` signal for 60 ticks after
a correct code; the `securitycraft:iron_door_reinforced` block with its item,
models, textures and loot; the keypad→door redstone link; `/securitycraft
info|code` for the harness.

**Out (gaps):** owner and allowlist/denylist (see "Deviations"), the smart and
harming modules, the disguise module and its tint, the `disabled` option,
waterlogging, wall/floor/ceiling mounting, the passcode-protected
chest/barrel/furnace/smoker, the keypad door and trapdoor, the universal block
remover/reinforcer and the whole "reinforced block" family, cameras, mines,
lasers, scanners, alarms, display cases, the frame/disguise conversion, the
reinforced door's recipe (upstream reinforces an iron door in world), and the
owner-only breaking speed.

## Behaviour (from the files in FINDINGS.md)

| Area | Upstream | This port |
|---|---|---|
| code storage | random 16-byte salt + PBKDF2-HMAC-SHA1 (65,536 rounds); client SHA-512 first | random 16-byte salt + **SHA-256** of `salt ‖ digits`, hex in the block entity's NBT (`salt`, `code`) |
| code length | any digit string (the text box filters `\d*`) | 1–20 digits (`runtime::valid_code`) |
| set | only an owner, only when no code exists (set screen) | **anyone** when no code exists; no owner rule in this slice (gap) |
| check | everyone else; a match calls `activate` | the same; a match sets `powered` for 60 ticks |
| screen | 176×186 panel, drawn widgets, censoring text box | a 176×226 mod panel drawn with `native_fill`/`native_text`; keys are hit-tested regions sent as **menu buttons**; the code display shows one `*` per typed digit |
| wrong code | message + optional harming/smart module | the screen says "Wrong code"; no message module, no cooldown |
| powered keypad | a right click does nothing | the same (`right_click_block` cancels) |
| signal | `POWERED` → 15 on every face, `updateIndirectNeighbors`, off after `signalLength` (60) | `native_block_signal` answers 15 while `powered=true`; the block entity ticks the countdown and clears the state with `native_set_block_property` (a neighbour update, so the door closes) |
| placement facing | faces the placer, 6-way (`FACING` + `ROTATION`) | faces the placer, **4-way** (the barrel template's `facing`), see deviations |
| reinforced door | a vanilla door with owner data and an activator list; `open` from an active keypad or a 15 signal | `minecraft:iron_door` template: `open`/`powered` from `hasNeighborSignal`, which reads the keypad's mod signal; iron-door sounds, double block, drop from the lower half |
| door breaking | owner-only speed (`destroySpeed` -1 for strangers) | hardness 5.0 for everyone (gap) |

## Hooks

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15) | `block_entity.rs` | The keypad gets a block entity. |
| `block_entity_ticks` (15) | `block_entity_ticks.rs` | It ticks. |
| `block_entity_container` (16/17) | `block_entity_container.rs` | One hidden slot, no faces, menu `securitycraft:keypad`. |
| `block_entity_tick` (17) | `block_entity_tick.rs` | Signal countdown, `powered` off, menu data slots. |
| `right_click_block` (21) | `right_click_block.rs` | Sets the screen's mode; a powered keypad is cancelled. |
| `screen` / `screen_draw` (19) | `screen.rs`, `screen_draw.rs` | The panel, the code display and the twelve keys. |
| `screen_click` (20) | `screen_click.rs` | A key press becomes a menu button id. |
| `menu_button` (20) | `menu_button.rs` | The server applies the key (digit/backspace/submit), stores/sets, powers the keypad. |
| `block_signal` / `is_signal_source` (38) | `block_signal.rs`, `signal_source.rs` | 15 on every face while powered; every keypad state is a signal source. |
| `state_for_placement` (164) | `state_for_placement.rs` | The keypad faces its placer. |
| `register_commands` / `perform_command` (8) | `register_commands.rs`, `perform_command.rs` | `/securitycraft info|code` (harness). |

All injections resolve against the pin's `native-hooks.json` (`plan.json`: 14
injections, 2 modules).

## Content carried (from the jar, no invented assets)

- `data/securitycraft/horizon/block/keypad.json` — template
  `minecraft:barrel` (its class opens a menu on use), hardness 5.0, explosion
  resistance 3.6e6 (upstream `Float.MAX_VALUE`), metal sounds, `requires_tool`,
  declared `powered` boolean, default `facing=north, open=false,
  powered=false`, **no** block item (the key panel places it).
- `data/securitycraft/horizon/block/iron_door_reinforced.json` — template
  `minecraft:iron_door`, the same hardness/sounds/tool rule, block item on (the
  double-high placement).
- `data/securitycraft/horizon/item/keypad_item.json` — template
  `minecraft:iron_ingot`, `place_block` the keypad.
- `data/securitycraft/horizon/menu/keypad.json` — 1 slot, 6 data slots.
- `data/securitycraft/horizon/creative_tab/securitycraft.json` — the two items.
- Loot tables in the 26.3 shape (a pool-level `condition`, an entry-level
  `block_state_property` for the door's lower half).
- The key panel recipe (the jar's, with `minecraft:stone_button` standing in for
  the not-yet-carried `securitycraft:reinforced_stone_button`).
- `minecraft:mineable/pickaxe` for both blocks (the jar's tag lists them).
- Blockstates/models/textures: the jar's keypad blockstate trimmed to the
  states this block has (4 facing × 2 open × 2 powered = 16 variants) with the
  cube model built from the jar's `keypad_unactive`/`keypad_active` textures;
  the jar's eight reinforced-door models unchanged (their vanilla parents still
  exist in 26.3); the six jar textures; the 26.3 `items/` files and two item
  models; `en_us.json` entries.

## Deviations (deliberate)

1. **No owner, no allowlist/denylist.** Upstream sets an owner on placement and
   only the owner may set the code or break the block; a stranger gets "not set
   up" and cannot open the set screen. This slice lets **anyone** set the code
   of a keypad that has none (recorded as a gap, not as behaviour).
2. **SHA-256 over a 16-byte salt, not PBKDF2.** Same shape (one random salt per
   keypad, a one-way digest, no plaintext digits saved) without a 65,536-round
   key derivation on the server thread; the in-game digest comes from the
   level's random (`native_level_random`).
3. **4-way facing, not 6-way.** The barrel template supplies a horizontal
   `facing` plus `open`; the keypad therefore mounts on walls, not on ceilings
   or floors. The jar's up/down blockstate variants and `rotation` property are
   not carried.
4. **One entry per keypad, not per player.** `native_menu_button` names the
   block entity but not the player, so the typed entry and the screen mode live
   in the block entity. Two players on one keypad at the same time share the
   entry (a single keypad is normally one player's).
5. **The typed entry is saved in the block entity** (`entry`) until Enter or the
   next open, where upstream keeps it in the client text box only. It is the
   digits typed so far, never the stored code.
6. **Explosion resistance 3.6e6**, not `Float.MAX_VALUE` (backwards compatible
   with every vanilla explosion, and a finite f32 in the content JSON).
7. **The reinforced door's item id is the block id**
   (`securitycraft:iron_door_reinforced`); upstream's separate item id
   `securitycraft:door_indestructible_iron_item` is not reproduced because a
   content block's item takes the block's id and keeps the double-high
   placement. Its model and texture are the jar's.
8. **No cooldowns.** Upstream's 250 ms per-player check cooldown and the smart
   module's keypad cooldown are not carried (no modules in this slice).
9. **No owner-only breaking speed**: the door and keypad are ordinary hardness
   5.0 blocks for everyone; upstream's blocks are effectively unbreakable by
   strangers.
10. **No sounds** on the keypad (upstream plays the UI button click per digit);
    the door keeps the vanilla iron-door sounds from its template.

## Harness additions

`/securitycraft info <x> <y> <z>` prints one chat line:

```
securitycraft keypad 21 101 0 code=set entered= mode=check signal=58 powered=true status=3 door=20,101,0:open
```

(`code` set/none, the masked entry, the screen mode, the signal left, the
powered state, the last status and the adjacent reinforced door's position and
`open` state). `/securitycraft code <x> <y> <z> <digits>` (op level 2) sets a
code without the screen. Both exist because a headless harness cannot type into
a text field; they are recorded as harness additions, not gameplay (precedent:
`/enderstorage info`, `/ironfurnaces info`).

## Notes for the next slice

- The slot runner rebuilds in the same lease is **not optional**:
  `build-slot.sh run|e2e` shares `target/` between ports, and a probe that only
  ran `horizon.py ctl` ran another port's `horizon-client` (the content loaded,
  the native hooks did not, `/securitycraft` was unknown). Always
  `horizon.py build` + `horizon.py ctl` + the scenario in **one** lease.
- The mixin planner rejects `//!` inner doc comments in **every** module
  (`E_MODULE`: "support module source cannot have file attributes or a
  shebang"); module headers are `//` comments.
- `horizon_world::level::place::PlacementContext` is private; the public path is
  `horizon_world::level::PlacementContext`.
- A content manifest has no `ported-from` field (`id, name, version,
  description, authors, api, side, capabilities, namespaces, experimental`);
  the licence belongs in this file.
- `ctl ui`'s `gui_size` is float (`[427.0,240.0]`): `jq … | floor` before
  arithmetic. A mod screen's `ctl screen` label is `container`, not
  `inventory`; its panel origin is `((gui_w-176)/2, (gui_h-226)/2)`.
- `tools/test_template.py` is the template's raw-hook fixture: it needs a
  workspace-root `native-hooks.json`, which the scaffold does not ship, so it
  rejects a named-hook bundle with `E_HOOK_CATALOG`;
  `tools/test_stage.py` points at the non-existent `ports/horizon`. Both are
  the template's stale scripts; the named-hook ladder used here is
  `tools/horizon.py stage → check → test → build`, the campaign's command set.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Verification

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`** (game.rs
`64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f`), mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Every build/test/e2e
step ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`; no
bare `cargo`, no `--target-dir`. Evidence:
`ports/_shared/evidence/securitycraft/{check,test,build,e2e,selftest,hmod-load}.log`.

### Ladder

| Step | Command (via the slot runner) | Result |
|---|---|---|
| runtime TDD (red 1) | `cargo test --offline` with `todo!()` stubs | **0 passed, 20 failed**, all "not yet implemented" |
| runtime TDD (red 2) | the eight mode/facing/submit tests added with stubs | **20 passed, 8 failed** (the new features missing) |
| runtime TDD (green) | `cargo test --locked --offline` | **28 passed**, 0 failed |
| fmt / clippy | `cargo fmt --check`, `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| stage + check | `python3 tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** (`check.log`) |
| embedded test | `python3 tools/horizon.py test` | `test result: ok. **28 passed**; 0 failed; 1240 filtered` (`test.log`) |
| fast build | `python3 tools/horizon.py build` | `cargo_exit: 0` (8m 22s, `build.log`) |
| e2e | `horizon.py build` + `horizon.py ctl` + `tools/e2e.sh` in one lease | **PASS**, `EXIT:0` (`e2e.log`) |
| fail-capability | `SELFTEST=1 tools/e2e.sh` | **EXIT:1**, "SELFTEST: expected the door OPEN after a wrong code" (`selftest.log`) |
| content check | `cargo horizon-mod check content/securitycraft.hmod` | `securitycraft 0.1.0 (mod, API 0.2, both)`, "no code: content only" |
| load-check | `tools/hmod-load.sh` on the shared `1d63e0a5` game | **PASS**, `EXIT:0`, with the negative control (`hmod-load.log`) |

### In-game scenario (quoted from `e2e.log`)

```
door: securitycraft:iron_door_reinforced[facing=north,half=lower,hinge=right,open=false,powered=false]
keypad: securitycraft:keypad[facing=south,open=false,powered=false]
after set: securitycraft keypad 21 101 0 code=set entered= mode=check signal=0 powered=false status=2 door=20,101,0:closed
after wrong code: securitycraft keypad 21 101 0 code=set entered= mode=check signal=0 powered=false status=1 door=20,101,0:closed
after right code: securitycraft keypad 21 101 0 code=set entered= mode=check signal=58 powered=true status=3 door=20,101,0:open
after the signal: securitycraft keypad 21 101 0 code=set entered= mode=check signal=0 powered=false status=3 door=20,101,0:closed
unlinked keypad: securitycraft keypad 24 101 0 code=set entered= mode=check signal=58 powered=true status=3 door=none
fresh keypad: securitycraft keypad 24 101 4 code=none entered= mode=set signal=0 powered=false status=0 door=none
after an empty submit: securitycraft keypad 24 101 4 code=none entered= mode=set signal=0 powered=false status=4 door=none
PASS: the key panel placed a keypad and the reinforced door item placed both halves; a fresh keypad set a code without opening anything; a wrong code left the reinforced door closed; the right code powered the keypad and opened the door beside it; the door closed when the signal ran out; a second keypad with the same code but no door powered on and opened nothing; an empty submit stored nothing; and a hand click cannot open the reinforced door
EXIT:0
```

The input path is the real one: `ctl interact` opens the panel, `ctl click-at`
on the drawn keys reaches `native_screen_click`, the button id reaches
`native_menu_button` on the server, and the code is typed as 1-2-3-4 + OK.

**Differentials / controls:** the same code on a keypad with **no door beside
it** powers the keypad (`powered=true`, `signal=58`) and opens nothing
(`door=none`); a **wrong** code leaves the linked door closed; a **fresh**
keypad opens in set mode and an empty submit stores nothing (`code=none`,
`status=4`); a hand click on the reinforced door opens no screen and leaves it
closed (an iron door cannot be opened by hand). The door's open/closed state is
read back with `ctl block`, not from the mod's own message.

**Fail-capability:** the SELFTEST run asserts the door is open after the wrong
code and fails exactly there, so the "door closed" assertions are not vacuous.

### Artifacts

| Artifact | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/securitycraft.hmod` | 12,270 B | `6a6861d7e82d9a9a96567e11b32c480758da387a2a9d7559ed7681a9880eec8e` | 33 |
| `securitycraft.hnmod` (2026-11-10, content embedded) | 26,658 B | `7ee2aeaeeec98b01b72bbb40bf79b516b97cd1489dd042e175f3ced96e87a910` | 20 |
| `securitycraft.hnmod` (2026-11-09, before the embedding) | 18,317 B | `1fd6057f7774033241b012bb9d312b6d27c687e78108b78df4a20d2726b7eec6` | 18 |

`horizon-mod check`: `securitycraft 0.1.0 (mod, API 0.2, both)`, 12,270 B, the
same hash, "no code: content only". The load-check places and gives the ids on
the shared game with `--dev-mod content/securitycraft.hmod`, and the same ids
are unknown without the package (`Unknown block type 'securitycraft:keypad'`),
so the check is not vacuous. The `.hnmod`'s `horizon-native.json` pins
`horizon` = `1d63e0a5…` and `mixins` = `5a64af93…`.

### Unified `.hmod` (2026-11-09) — content and code in one file

The pin's `docs/api/mod-files.md` and `docs/native-mods.md` ("One `.hmod` for
content and code") say a `.hmod` may hold **both** halves: `horizon-mod.toml`,
`assets/` and `data/` are read as content (`horizon_mods::Content`), and
`horizon-native.json` + `mixins/<id>/` as native code (`Bundle`); the two
manifests must name the same mod id. `horizon_mods::index::native::holds`
already recognises "a `.hmod` holding native code".

`tools/package_unified.py` builds that file deterministically from this tree
(content entries first, then `horizon-native.json` + `mixins/securitycraft/`,
fixed zip timestamps, sorted). `--check` rebuilds in memory and fails when the
file on disk is not exactly that build, so the hash below is re-provable:

| Artifact | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `securitycraft-unified.hmod` | 30,565 B | `3f1a91eb93ae042425b8e5aed4461364e46e55cb88860120881ab3690121fc33` | 51 (33 content + 18 native) |

- `cargo horizon-mod check securitycraft-unified.hmod` **refuses it by design**:
  `it has native code (horizon-native.json): it is a native-mod`.
- `cargo horizon-mod check … --kind native-mod` **passes**:
  `securitycraft 0.1.0 (native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3)`,
  same size/hash, and it reports the native-mod source warning instead of
  "no code: content only" (it is a native mod now).
- Evidence: `evidence/securitycraft/check-unified.log`.

**In game** (`evidence/securitycraft/e2e-unified*.log`): the whole slice runs
with `--dev-mod securitycraft-unified.hmod` — the keypad and reinforced door
come from the one file, and the scenario (set a code, wrong code keeps the door
closed, right code opens it, the signal ends and it closes, the unlinked-keypad
differential, the empty-submit case, the hand-click control) passes with
`EXIT:0`. The native sources inside the file are byte-identical to the
`mixins/securitycraft/` the same lease compiled (the driver checks every entry
and prints its SHA-256), so the binary under test contains exactly the code the
file carries. `SELFTEST=1` fails at its own assertion (`EXIT:1`), and
`CONTROL=1` (no package) shows `unknown item securitycraft:keypad_item`,
`Unknown block type 'securitycraft:keypad'` and no `/securitycraft` reply.

**Registry and launcher (not tested in a launcher; read from the pin's code).**
The file cannot be *listed or installed* today:

- `cargo horizon-mod registry build` accepts a `native-mod` entry only with a
  `.hnmod` file (`index.rs` `Kind::extension`; `is not a plain file name for a
  native-mod`), and a `kind = "mod"` entry refuses the unified file because
  `it has native code … it is a native-mod`. The same entry with
  `securitycraft.hnmod` builds the index, so the refusal is the extension, not
  the content (`evidence/securitycraft/registry-check-unified.log`).
- The launcher reads native mods only from `horizon-native-mods/*.hnmod`
  (`native_mods.rs:621`, `native_status.rs:247`); a registry install rewrites a
  native-mod download to `<id>.hnmod` before it gets there
  (`browser/horizon.rs:848`). "Add from GitHub" refuses a lone `.hmod`
  ("installing `.hmod` files from a link is not built",
  `docs/native-mods.md`; `github_mods.rs` keeps only the `.hnmod` asset).
- So the format works in the game, but **no registry/launcher path installs a
  native `.hmod` at this pin**. Until `Kind::extension`/the launcher accept it,
  a two-half port still needs API 145 `native_blocks`/`native_items` (declare
  the content in code) or waits for upstream to switch, as `AGENT_PLAYBOOK.md`
  §1.0 says.

### Embedded content (AE2 pattern, 2026-11-10) — the single listable artifact

The registry stores **one file per project** (playbook §1.0), so a port with
both halves could not be listed faithfully, and the unified `.hmod` route is
closed (`Kind::extension` refuses `native-mod` with `.hmod`; the launcher reads
`horizon-native-mods/*.hnmod` only — §"Unified `.hmod`" below). This bundle now
carries its content, exactly as the merged AE2 entry and `charginggadgets`
(PR #526) do:

- **`mixins/securitycraft/data/securitycraft-content.hmod`** is the built
  `content/securitycraft.hmod`, byte for byte (12,270 B,
  `6a6861d7…ec8e`). `tools/package.py` packs `data/**` as
  `mixins/<id>/data/…` (top-level `.rs`/`.toml` plus everything under `data/`)
  with the same fixed zip timestamps, so a rebuild reproduces the SHA-256 (two
  consecutive runs: identical), and it **refuses** a
  `data/<id>-content.hmod` that is not byte-identical to `content/<id>.hmod`,
  so a rebuilt content mod can never ship stale inside the bundle.
- **`mixins/securitycraft/embedded_content.rs`** with the `embedded-content`
  `[[injection]]` (`target = "horizon_client::game::mods::content_root"`,
  `kind = "HEAD"`, `cancellable = true`, bindings `root`/`enabled`/`dev`)
  compiles the file in with `native_data!` (API 41), writes it to
  `.horizon-cache/securitycraft-embedded-content.hmod` when the copy on disk
  differs, appends it to the dev-mod list and returns its `content_root`. The
  helper calls `content_root` once more and stops at its own
  `dev.contains(&file)` guard, so the second call runs the vanilla body — the
  same one-level recursion AE2 uses. An unwritable cache logs a warning and
  `Continue`s (no content) instead of failing the game.

Verified in game with **only the bundle** (`EMBEDDED=1`, no `--dev-mod`): the
keypad and reinforced-door scenario (phases 1–8) passes, and the file the
helper wrote is byte-identical to `content/securitycraft.hmod`. The control
(`EMBEDDED=1 CONTROL=1`) launches the same binary with `.horizon-cache`
replaced by a plain file, so the helper refuses its content: `ctl give
securitycraft:keypad_item` answers `unknown item`, `setblock …
securitycraft:keypad` is refused and leaves `minecraft:air`, and
`/securitycraft` answers nothing — the ids are unknown without the embedded
file, so the PASS is not vacuous.

Harness fact worth keeping: this port's e2e passes `--game-dir`, and the game
`set_current_dir`s into it (`game.rs`), so the helper's `.horizon-cache` is
`<game dir>/.horizon-cache/…`, not the launch directory — a control that blocks
`$SCRATCH/.horizon-cache` would pass vacuously (measured: the first attempt did
exactly that). `charginggadgets` has no `--game-dir`, which is why its control
blocks `$SCRATCH/.horizon-cache`.

`tools/e2e-embedded.sh` is the driver (one `build-slot.sh e2e` lease):
package/entry identity + `cargo horizon-mod check --kind native-mod` → ctl →
embedded run with its control → selftest. Evidence:
`ports/_shared/evidence/securitycraft/{e2e-embedded.log,e2e-embedded-selftest.log,embedded-driver.log}`.

| Step (this slice) | Result |
|---|---|
| `cargo fmt --all --check`, `rustfmt --check mixins/securitycraft/embedded_content.rs` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings (`clippy-embed.log`) |
| `cargo test --locked --offline` | **28 passed**, 0 failed (`unit-tests-embed.log`) |
| `horizon.py stage` / `check` / `test` | stage_ok; `cargo_exit: 0`; **28 passed / 1240 filtered** (`stage.log`, `check.json`, `check.log`, `test.log`, `htest.json`) |
| `horizon.py build` (fast) | `cargo_exit: 0` (`build.log`); the e2e lease rebuilds game + `ctl` (`ctl.log`) |
| `tools/package.py` → `securitycraft.hnmod` | 26,658 B, `7ee2aeae…a910`, 20 entries |
| `cargo horizon-mod check --kind native-mod` | `securitycraft 0.1.0 (native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3)`, 26,658 B, same hash |
| `tools/e2e-embedded.sh` (`EMBEDDED=1 CONTROL=1`, no `--dev-mod`) | **PASS + `EXIT:0`** (`e2e-embedded.log`), control leg shows the ids unknown |
| `EMBEDDED=1 SELFTEST=1` | **FAIL + `EXIT:1`** at the door assertion (`e2e-embedded-selftest.log`) |
| `check-e2e-logs.py securitycraft --strict` | `logs=5 ok=5`, exit 0 |
| `upstream_ready.py securitycraft` | **READY**, exit 0 (the split-port blocker is gone) |

Quoted from `e2e-embedded.log`:

```
== control: the same build with no embedded content file ==
  launch: no --dev-mod; the bundle carries its own content
give: {"error":"unknown item securitycraft:keypad_item"}
setblock: {"chat":["Unknown block type 'securitycraft:keypad'",…],"ok":true}
block at 16,100,3: minecraft:air
  without the embedded file: no keypad item, the keypad block is unknown
  embedded content: 12270 B at …/game/.horizon-cache/securitycraft-embedded-content.hmod, identical to content/securitycraft.hmod
after the right code: securitycraft keypad 23 101 4 code=set entered= mode=check signal=58 powered=true status=3 door=22,101,4:open
PASS: the key panel placed a keypad and the reinforced door item placed both halves; …
EXIT:0
```

This makes the `.hnmod` the single listable artifact of this port; it does not
by itself publish it (licence approval is still required, ARCHITECTURE.md §8).
`securitycraft-unified.hmod` stays in the port as the evidence of the
abandoned route only — it must not be submitted.

### Mode B rebuild (2026-11-09) — artifact made provably current

Pin unchanged (`1d63e0a5…` / `5a64af93…`); the tree was rebuilt, repackaged and
re-verified so the delivered files are the working version. All steps through
`ports/tools/build-slot.sh e2e --job MB-securitycraft --port ports/securitycraft
--horizon …/pins/horizon-1d63e0a` with `CARGO_INCREMENTAL=0`.

| Step | Result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** |
| `cargo test --locked --offline` | **28 passed**, 0 failed |
| `horizon.py stage` / `check` / `test` | stage_ok, `cargo_exit: 0`, **28 passed / 1240 filtered** |
| `tools/package.py` → `securitycraft.hnmod` | 18,317 B, `1fd6057f…eec6`, 18 entries |
| `cargo-horizon-mod build` → `content/securitycraft.hmod` | 12,270 B, `6a6861d7…ec8e`, 33 entries, "no code: content only" |
| `tools/e2e.sh` | **PASS + `EXIT:0`** (`evidence/securitycraft/e2e.log`) |
| `SELFTEST=1 tools/e2e.sh` | **FAIL + `EXIT:1`** (`evidence/securitycraft/e2e-selftest.log`) |
| `check-e2e-logs.py securitycraft --strict` | `logs=2 ok=2`, exit 0 |

The e2e script now also prints `EXIT:<status>` from its cleanup trap, so a
failing leg can never look like a pass. At that point `upstream_ready.py
securitycraft` reported the **campaign-level split-port blocker**
(`content/<id>.hmod` + `<id>.hnmod` is one file per registry entry) — no
artifact, pin or evidence blocker remained. **Resolved 2026-11-10** by the
embedding above: `upstream_ready.py securitycraft` is now **READY** (exit 0).

## Gaps (unchanged by this slice)

Owner/allowlist/denylist; the smart and harming modules; the disguise module;
the `disabled` option; waterlogging; ceiling/floor mounting; the passcode
chest/barrel/furnace/smoker, keypad door and trapdoor; the universal block
remover/reinforcer and the reinforced-block family; cameras, mines, lasers,
inventory and retinal scanners, the alarm, display cases; the frame and its
`IPasscodeConvertible`; the reinforced door's in-world recipe; the owner-only
breaking speed; and the mod's config file (`sendAllowlistMessage`,
`signalLength`, `disabled` are not configurable — the signal length is the
upstream default 60).

## Review repairs (PR #572, 2026-11-11)

Review: `ports/_shared/horizon-mods/reviews/pr-572-securitycraft.md` (COMMENTED + CHANGES_REQUESTED, 1 P1).

| Reviewer's item | This job |
|---|---|
| the PR body names the old `1fd6057f…` artifact | **orchestrator** (the entry and the file are the new digest) |
| artifact not staged | **orchestrator** (no write access) |
| **[P1] "do not persist typed passcodes in the saved block entity"** — `store_entry` wrote `entry.text()` into `entity.nbt` after every digit | **fixed** (see below) |

### the typed code never reaches the save

The digits typed into an open keypad now live in a process-local `TYPED` table keyed by
position, not in the block entity's compound. `store_entry` takes a position and cannot
touch an entity at all (the signature is the guard), `entry_at` reads it back, and the
screen's data slot still shows the typed length. Opening the screen, pressing Enter and a
wrong or right submission all clear it; a save holds only `code` (salted digest), `salt`,
`mode`, `status` and `signal`. A key written by the previous build is scrubbed on the
keypad's first tick (`scrub_legacy_entry`) instead of being read.

Tests (in `support.rs`, run as `game::securitycraft_state::tests`):
`the_typed_digits_live_outside_the_saved_compound` (a save/reload regression: the compound
never contains the typed code), `a_legacy_saved_entry_is_scrubbed_not_read`, and
`two_keypads_keep_their_own_digits`. The entry no longer needs the "never in plain text"
claim qualified — it is now true.

Verified: `python3 tools/horizon.py test` in a build slot — staged compile `cargo_exit: 0`,
**31/31** tests pass with the widened `game::securitycraft` filter, including
`game::securitycraft_state::tests::the_typed_digits_live_outside_the_saved_compound`,
`a_legacy_saved_entry_is_scrubbed_not_read` and `two_keypads_keep_their_own_digits`
(`ports/_shared/evidence/securitycraft/review-repairs-test.log`).

Verified after the change: `tools/e2e.sh` — **PASS + `EXIT:0`**: a fresh keypad sets a code
without opening anything, a wrong code leaves the door closed, the right code powers the
keypad and opens the reinforced door, the door closes when the signal runs out, a keypad with
no door powers and opens nothing, an empty submit stores nothing, and a hand click cannot open
the door. The readout shows `entered=` for a keypad that has typed nothing, i.e. the typed
digits are no longer in the saved compound
(`ports/_shared/evidence/securitycraft/e2e.log`).

Artifact: `securitycraft.hnmod` **29,414 B** `2fe90376c455bb5e9715c89900740da43f41d19659040749d81f27f12ddc9d39`
(content `securitycraft.hmod`).

## Migration to the SDK format (2026-11-18, SDK `0b14f8a`)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod` container, and
the registry closed this port's submission (**PR #839**) with *"republish this as a native `.hmod`
with the SDK from the next Nightly"*. The port is now one flat package; only the glue, the manifest,
the packaging and one carried data field changed.

- **Pin:** Horizon **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (nightly 2026-11-18), horizon-mixins
  **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, mod API **`0.2`** (SDK fetched into `.horizon/sdk/`
  by `cargo horizon-mod`, never packed).
- **Version:** the port's own **0.2.0** (the retired bundle was 0.1.0; a resubmission is always a
  new version).
- **Class + tier** unchanged: class **content**, tier **deep** in the pack's row, delivered as the
  **logic + machines** shape the two blocks need (block entity, menu, screen, redstone) — no new
  feature, no re-tiering.

### File map (created / deleted)

| Created | What it holds |
|---|---|
| `horizon-mod.toml` | the only manifest: `id = "securitycraft"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` with the exact `game`/`horizon`/`mixins` |
| `Cargo.toml` | the crate: `horizon-mod-api` from `.horizon/sdk/` + `sha2 = "0.10"` (the carried rules module's one dependency) |
| `Cargo.lock` | the resolved pins for `sha2`/`cpufeatures`; **inside the package** (see trap 2) |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` (`register`, `register_commands`, `run_mod_command`) and the keypad's `BlockPart`/`BlockEntityPart` |
| `src/support.rs` | the game glue: saved-data helpers, the block entity's payload, the screen's rectangles/drawing, the menu buttons, redstone, placement, the two harness commands, and its 4 tests |
| `src/runtime.rs` | the retired `runtime.rs`, **byte-identical** (SHA-256 `1b2744a909c36c98d4c78dc09a28a060709c1fa17543ffed571f5c384fb1756a`, `cp` then `cmp`, and the same hash inside the package), 28 tests |
| `assets/`, `data/` | the retired `content/assets` + `content/data`, unchanged except one field (trap 1) |
| `LICENSE-NOTICE.md` | licence provenance, inside the package |
| `securitycraft.hmod` | the delivered package (port root) |
| `.gitignore` | `/target`, `/.horizon` |

| Deleted | Why | Where it went |
|---|---|---|
| `mixins/securitycraft/` (`horizon-mixin.toml`, `support.rs`, `runtime.rs`, one helper per hook, `data/securitycraft-content.hmod`) | the plan/injection schema and per-hook helpers are not part of any SDK package | `runtime.rs` → `src/runtime.rs` (byte-identical); the hook bodies → `src/lib.rs` Parts + `src/support.rs`; the glue's 3 tests → `src/support.rs` |
| `content/` (`horizon-mod.toml`, `assets/`, `data/`, `target/`, `securitycraft.hmod`) | a second package; the flat package carries content itself | `assets/` + `data/` at the package root; the built `.hmod` is evidence |
| `securitycraft.hnmod`, `securitycraft-unified.hmod` | the retired container (and the abandoned unified route) | evidence only — neither must be submitted |
| `compatibility.json` | the pin is `[native]` in `horizon-mod.toml` | evidence |
| `rust-toolchain.toml` | not part of any new package; the SDK's toolchain decides | evidence |
| `tools/horizon.py`, `package.py`, `package_unified.py`, `notice.py`, `test_stage.py`, `test_template.py`, `audit/`, `hmod-load.sh`, `e2e-embedded.sh`, `e2e-unified.sh` | the staged/mixin build and packaging machinery; there is no stage/plan/pack step any more | evidence |
| `.github/` (the template fixture workflow) | it clones horizon-mixins and runs `tools/test_template.py`, neither of which exists at this pin | evidence |

Kept on purpose: `tools/e2e.sh` (the in-game scenario is still the only behaviour oracle; a
`STATUS` header explains the launcher-built game it now needs), `FINDINGS.md`, `notes.md`,
`LICENSE`, `README.md` (rewritten for the flat layout), `VALIDATION.md`, `AGENTS.md` (rewritten).

### Helper → hook table

| Retired `[[injection]]` | SDK 0.2 (where) | Mapping |
|---|---|---|
| `block_entity` HEAD (`entity: &mut Option<(String, Compound)>`) | `BlockEntityPart::block_entity` → `Option<Compound>` (`src/lib.rs`) | the block entity type owns the id now (`r.block_entity_type("keypad").blocks(&[…])`), so the hook only supplies an empty compound |
| `block_entity_ticks` HEAD (`ticks: &mut bool`) | `BlockEntityPart::block_entity_ticks` → `event.ticks = true` | unchanged |
| `block_entity_container` HEAD (`container: &mut Option<ModContainer>`) | **no hook**: `r.block_container(&block, ModContainer { … })` in `Hooks::register` | one hidden slot, no faces, `menu = Some("securitycraft:keypad")` |
| `block_entity_tick` HEAD (game, pos, `entity: &mut ModBlockEntity`) | `BlockEntityPart::block_entity_tick` (`event.pos`, `event.entity: ModBlockEntityView`) | the same countdown; `powered=false` through `host.set_block_property`; `event.entity.data` carries the six data slots |
| `right_click_block` HEAD (`canceled`) | **`BlockPart::use_item_on` and `use_without_item`** returning `Option<InteractionResult>` | sets `mode`/`status` in the saved data when unpowered and answers `None` (the content block's own use opens the menu); a powered keypad answers `Some(Fail)`, which ends the click as the retired `canceled = true` did |
| `screen` HEAD (`screen: &mut Option<NativeScreen>`) | `BlockEntityPart::screen` → `Option<Screen>` | the same 176×226 panel, one hidden slot, inventory at y=140 |
| `screen_draw` HEAD (`game`, `batch: HudBatch`, `origin`) | `BlockEntityPart::screen_draw` with `event.draw: &mut dyn DrawContext` | `native_fill`/`native_text`/`native_text_width` → `DrawContext::fill`/`text`/`text_width` (ARGB, so the retired RGB colours gained `0xff`) |
| `screen_click` HEAD (`button: &mut Option<i32>`) | `BlockEntityPart::screen_click` → `Option<i32>` | the same 12-rectangle hit test, now through `support::key_rect` |
| `menu_button` HEAD (game, pos, `entity`, `id`) | `BlockEntityPart::menu_button` (`event.context.id`, `event.context.pos`, `event.entity`) | unchanged: digit/backspace/submit, then `powered=true` and 60 ticks' signal on `Open` |
| `block_signal` HEAD (`signal: &mut i32`) | `BlockPart::block_signal` (`BlockSignalEvent { state, signal, … }`) | 15 on every face while `powered=true` |
| `is_signal_source` HEAD (`source: &mut bool`) | `BlockPart::is_signal_source` (`IsSignalSourceEvent { state, source }`) | every keypad state is a signal source |
| `state_for_placement` HEAD (`answer`) | `BlockPart::state_for_placement` → `Option<Option<BlockState>>` | `level.state_with_property(state, "facing", facing_from_yaw(context.yaw))` |
| `register_commands` HEAD (`commands: &mut Vec<NativeCommand>`) | `Hooks::register_commands` (`event.commands: Vec<Command>`) | `/securitycraft info <pos:block_pos>` and the op-level-2 `/securitycraft code <pos:block_pos> <code:word>` |
| `perform_command` HEAD (raw line) | `Hooks::run_mod_command` (parsed `CommandInvocation`) | the same two usages, read through typed arguments; feedback through `host.commands().feedback` |
| `embedded-content` (`content_root` HEAD) | **gone** | the flat package carries `assets/`/`data/` inside its own `.hmod`; the cache write, the `native_data!` blob and the control leg have no purpose |

`runtime.rs` is untouched: `valid_code`, `hash_code`, `new_salt`, `submit`, `after_submit`,
`button`, `Mode`, `Action`, `Entry`, `tick_signal`, `facing_from_yaw`, `DEFAULT_SIGNAL_TICKS`,
`BUTTON_BACKSPACE`, `BUTTON_SUBMIT`, `MAX_CODE_LEN` are called exactly as before; the new glue only
converts between the SDK's types and theirs (`Compound` keys through `Tag::String`, the level's
`LegacyRandom` for the salt, `ModBlockEntityView` for the payload).

### Traps this migration cost

1. **The content schema changed one field name.** `cargo horizon-mod build` refused
   `data/securitycraft/horizon/item/keypad_item.json`:
   `unknown field 'place_block', expected one of 'template', 'max_stack_size', 'rarity', 'components', 'crafting_remainder', 'block', 'arm_pose'`.
   An item that places a block now says `"block": "securitycraft:keypad"`. Nothing else in `data/`
   or `assets/` needed a change (blockstates, models, textures, loot, recipe, tag, lang and the
   `horizon/menu/keypad.json` menu all validated as carried).
2. **A registry dependency is the first in an SDK port, and it ships its lock.** The carried
   `runtime.rs` imports `sha2`, so the crate needs `sha2 = "0.10"` (resolved to 0.10.9, cached on
   this box). `cargo horizon-mod build` then packs `Cargo.lock` — the opposite of `imfast`'s trap 5:
   there the lock changed a 7,570 B artifact into 23,133 B of *wrong* versions, while here the lock
   is this package's only record of its pins and belongs inside it. The launcher builds with
   `horizon-launcher native-build … `, and a registry dependency additionally needs
   `horizon-launcher native-lock .` on the machine that builds the game — recorded as a gap, because
   no launcher is installed here.
3. **A `&dyn LevelHost` has no block metadata.** `block_signal`/`is_signal_source` are given a
   `LevelHost`, not a `Host`, so `host.block_states()` (the `BlockMetadata` table a property read
   needs) is out of reach. `powered` is therefore read from the state's own canonical form
   (`level.state_string(state)` contains `powered=true`) — honest, and exact for this block, but a
   gap worth an upstream issue: *"a `LevelHost` block-property read, so a state-only hook does not
   parse a serialised state"*.
4. **Command argument kinds changed.** `<code:string>` is not a kind at API 0.2 (`word`, `text`,
   `integer`, `number`, `boolean`, `player(s)`, `entity/ies`, `item`, `block_pos`, `vec3`, …); the
   digit string is a `<code:word>`.
5. **`build-slot.sh` does not `cd` into the port** and its own staged steps do not apply to an SDK
   package: it is used as a **lease only** (`--step test|lint|build|check -- bash -c 'cd
   ports/securitycraft && …'`). Never a bare `cargo`, never `--target-dir`.
6. **`cargo horizon-mod build` packs root files.** `README.md`, `LICENSE`, `LICENSE-NOTICE.md` and
   `Cargo.toml` go in as they are, so a stale README describing `mixins/` would ship inside the
   package; rewriting it for the flat layout is part of the migration, not a nicety.
7. **`capabilities` is a manifest field, but the check report prints `"capabilities":[]`** for a
   native mod even though this package really registers blocks — do not "fix" the manifest from the
   report (the same trap `repeatable_trial_vaults` recorded).
8. **`.horizon/sdk` can be copied instead of fetched.** The pinned SDK is cached in the campaign at
   `ports/_shared/sdk/0b14f8a/` and unpacked in every migrated port; `cp -a ../imfast/.horizon/sdk
   .horizon/sdk` gave the tool its `SDK 0b14f8a … in ./.horizon/sdk` line with no network.
9. **The retired e2e cannot be re-run here.** It needs a launcher-built game carrying this package
   (`horizon-launcher native-build … --trust-native-code`) and there is no game build at `0b14f8a`
   (the shared binaries are at `1d63e0a5` and `41f822b6`), so `tools/e2e.sh` is kept with a
   `STATUS` header and the 0.1.0 in-game run stands as the behaviour's evidence.
10. **Two builds give the same bytes.** `cargo horizon-mod build` twice: 43,969 B, SHA-256
    `5c7423f9…e6dc` both times (deterministic zip timestamps).

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

### Verification (SDK `0b14f8a`)

Every step ran in a `ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`, locally
(`BUILD_LOCAL=1`), on the box's rustc 1.97.0; logs in
[`../_shared/evidence/securitycraft/sdk-0b14f8a/`](../../_shared/evidence/securitycraft/sdk-0b14f8a/).

| Check | Command | Result |
|---|---|---|
| Runtime carried | `sha256sum src/runtime.rs` + `cmp` with the retired file | `1b2744a9…56a`, **byte-identical** (`runtime-carry.log`), and the same hash for `unzip -p securitycraft.hmod src/runtime.rs` |
| Rules tests first | `cargo test --offline` before any glue existed | `28 passed; 0 failed` |
| Tests with the glue | `cargo test --offline` | **32 passed**, 0 failed (28 runtime + 4 glue) (`sdk-sc-test.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-sc-fmt.log`) |
| Lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-sc-clippy.log`) |
| Package | `cargo horizon-mod build` ×2 | `target/horizon-mod/securitycraft.hmod`, EXIT 0, identical bytes twice (`sdk-sc-build.log`, `sdk-sc-build2.log`) |
| Delivered | `cp target/horizon-mod/securitycraft.hmod ./securitycraft.hmod` | **43,969 B**, SHA-256 `5c7423f9702e3cc2e0dca45bd3947c489b646231f2a83aebbb338c422d34e6dc`, **42 entries** (`horizon-mod.toml`, `Cargo.toml`, `Cargo.lock`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `src/{lib,runtime,support}.rs`, `uses.json`, 6 `data/` files, 23 `assets/` files) |
| Registry check | `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`, 43,969 B, same hash, **EXIT 0** (`sdk-sc-check.log`, `sdk-sc-check-json.log`) |
| Fits the pinned build | `cargo horizon-mod fits securitycraft.hmod --build nightly` | `SecurityCraft:` / `0b14f8a  made for`, **EXIT 0** (`sdk-sc-fits.log`) |
| What it uses | `cargo horizon-mod check … --uses` (packed `uses.json`) | **375 entries**, 50 of them hook surfaces: `block_entity`, `block_entity_ticks`, `block_entity_tick`, `screen`, `screen_draw`, `screen_click`, `menu_button`, `block_signal`, `is_signal_source`, `state_for_placement`, `use_item_on`, `use_without_item`, `register_commands`, `run_mod_command` — the ten Part methods plus the two command hooks (`sdk-sc-uses.json`) |
| Skills line | `python3 ports/tools/check-skills.py --notes ports/securitycraft` | see below |

### Gaps after the migration

1. **No in-game run at `0b14f8a`** — no launcher and no game build at this pin (trap 9). Everything
   else here is carried logic, type-checked glue and the package's own check; the first slice at a
   launcher-capable box runs `tools/e2e.sh` and records the transcript.
2. **No `BlockMetadata` from a `&dyn LevelHost`** (trap 3): `powered` is read from the serialised
   state. Issue-shaped, not worked around.
3. **A registry dependency needs `horizon-launcher native-lock .`** on the building machine
   (trap 2); nothing here can prove that step.
4. **The slice's own gaps are unchanged** from the retired bundle: owner/allowlist/denylist, the
   smart and harming modules, the disguise module, the `disabled` option, waterlogging,
   ceiling/floor mounting, the passcode chest/barrel/furnace/smoker, the keypad door and trapdoor,
   the universal block remover/reinforcer and the reinforced-block family, cameras, mines, lasers,
   scanners, the alarm, display cases, the reinforced door's in-world recipe, the owner-only
   breaking speed, and the mod's config file.

### Changelog for the resubmission

> Supersedes #839, closed because Horizon retired `.hnmod` (#17992). Changes: one flat SDK package
> with `api = "0.2"` and `[native]` pinned to `0b14f8a` / `e59da90`; the retired plan/injections
> are now `impl mod_api::Hooks` methods — the keypad is a `BlockEntityPart` attached to its own
> block entity type (`block_entity`, `block_entity_ticks`, `block_entity_tick`, `screen`,
> `screen_draw`, `screen_click`, `menu_button`) and its redstone/placement/use are `BlockPart`
> moments (`block_signal`, `is_signal_source`, `state_for_placement`, `use_item_on`,
> `use_without_item`); its container and content menu are declared once in `register`; the content
> half now ships inside the package (`assets/`, `data/`), so the embedded-content cache write is
> gone, with one field renamed by the schema (`place_block` → `block`); `runtime.rs` and its tests
> are byte-identical. Version 0.2.0.

## Review fixes (2026-11-18, PR #1197)

Two findings on the keypad:

1. **Plaintext PIN input was shared by XYZ alone.** The button handler ignored the acting player
   and closing a menu left the buffer behind, so another session at the same keypad, or a keypad
   at the same coordinates in another dimension, could modify or submit someone else's digits.
   **Fix:** every buffer is keyed by `Session { dimension, pos, player }`; the digits are cleared
   when the screen is opened (`on_use`), when a menu closes (`menu_closed`), when the block menu
   closes for any reason (`player_container_close`), and when the code command re-sets the keypad
   (`clear_keypad`). `data_slots`/`keypad_info` read only the acting player's session.
2. **No retry limit, and PINs as short as one digit.** **Fix:** `MIN_CODE_LEN = 4` in
   `valid_code` (both the set screen and `/securitycraft code`; a too-short submit is
   `Action::TooShort`, status 5), and a server-side `Throttle` per session: five wrong codes start
   a 10-second lockout (status 6) checked before any digest comparison, with a correct code
   resetting it. The budget deliberately outlives a screen close.

**Tests.** `a_session_is_the_world_the_player_and_the_keypad`,
`a_too_short_code_is_refused_before_any_comparison`,
`five_wrong_codes_lock_the_session_out_for_ten_seconds`, `a_correct_code_clears_the_budget`, and the
updated `a_code_is_four_to_twenty_ascii_digits`.

**In-game.** Phase 2 refuses a three-digit set (status 5); phase 3b types `12`, closes, reopens and
reads `entered=` empty; five wrong codes then make the right one status 6 (door closed); after the
lockout the right code powers the keypad and opens the door. Evidence:
`../_shared/evidence/securitycraft-e2e/e2e-launcher.log` (`PASS` + `EXIT:0`) and
`e2e-launcher-control.log` (`EXIT:1`); checker green.

**Runtime file.** `src/runtime.rs` now carries these new rules (minimum, `Action::TooShort`,
`Throttle`) on top of the retired bundle's byte-identical copy; its new sha256 is
`8f78aba553247a123cac17888e593b81c52c84c230d0f95de010c0b97ecb03f3`. The retired file's sha
(`1b2744a9…`) records the pre-review provenance.

**Package.** `securitycraft.hmod`, **0.2.1**, 45,092 B, sha256
`9bb21d4c189066b6b1720f8df235b3056e0d0047586b59f260a04c0aed490830`; two builds byte-identical,
`check` and `fits --build 0b14f8a` exit 0.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

## Slice 3 — the harming module (2026-11-21, SDK `0a875da`)

**Slice.** The gap line this job comes from is the port's own list: *"Owner/allowlist/denylist;
the smart and harming modules; the disguise module; the `disabled` option; …"*. From it this slice
takes **the harming module**, and the owner it needs to install one. Class **content**, tier
**deep**; the delivered shape is the port's existing **logic + machines** half (block entity, menu,
screen, redstone) plus one carried item and one carried damage type — no re-tiering.

**Why it fits and why it is proven with one player.** Upstream's `ModuleItem.useOn` installs a
module only for the block's owner, and `IPasscodeProtected.onIncorrectPasscodeEntered` hurts the
player for a wrong code when the harming module is present (`incorrectPasscodeDamage` = 4, two
hearts). Both effects belong to the keypad the port already ships, so the slice needs no camera,
renderer or dimension, and its in-game assertion (health 20 → 16 on a wrong code, unchanged
without the module) is fail-capable with a single player and a differential keypad. The owner gate
is a pure rule with a unit test; a *second* player cannot be driven by the single-player
`horizon-ctl` harness, so the in-game leg proves the owner installs and the differential, not the
stranger's refusal.

### Upstream read (`Geforce132/SecurityCraft`, branch `1.21.1`, commit `c4d35b0c8bf85c2a94c37a1f8ce37ef5ce1a6aa8`)

Behaviour only; this remains a clean-room Rust rewrite. Files read: `items/ModuleItem.java`
(`useOn`: `IModuleInventory`, `IOwnable.isOwnedBy`, `acceptsModule`, `!hasModule`, `insertModule`,
creative keeps the stack), `api/IModuleInventory.java` (`insertModule`, `hasModule`,
`dropAllModules`), `api/IPasscodeProtected.java` (`onIncorrectPasscodeEntered`: smart -> cooldown,
harming -> hurt then `closeContainer`), `misc/CustomDamageSources.java`
(`securitycraft:incorrect_passcode`), `ConfigHandler.java` (`incorrectPasscodeDamage` default 4),
`SCContent.java` (`harming_module`, `ModuleItem(…, HARMING, false)`), `blockentities/KeypadBlockEntity.java`
(accepted modules: allowlist, denylist, disguise, smart, harming). Data read from the jar:
`data/securitycraft/recipe/harming_module.json` (III / IPI / ISI, 7 iron + paper + arrow),
`data/securitycraft/damage_type/incorrect_passcode.json` (`exhaustion` 0.1, `message_id`
`securitycraft.incorrectPasscode`, `scaling` `when_caused_by_living_non_player`), the death-message
lang lines, the item model and the 16×16 texture.

### Red proof (TDD, before the rule existed)

`src/runtime.rs` gained the two tests first, with `may_install` and `wrong_code_damage` stubs that
kept the old behaviour; `cargo test --offline` in a slot then failed with:

```
---- runtime::tests::a_harming_module_damages_a_wrong_code stdout ----

thread 'runtime::tests::a_harming_module_damages_a_wrong_code' (73342) panicked at src/runtime.rs:656:9:
assertion `left == right` failed: two hearts, as upstream's incorrectPasscodeDamage default
  left: None
 right: Some(4.0)

---- runtime::tests::only_the_keypad_owner_may_install_a_module stdout ----

thread 'runtime::tests::only_the_keypad_owner_may_install_a_module' (73361) panicked at src/runtime.rs:671:9:
assertion failed: may_install(owner, 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef)
```

Log: `ports/_shared/evidence/securitycraft/sdk-0a875da-red.log`. The rules then became
`owner == Some(player)` and `harming.then_some(INCORRECT_PASSCODE_DAMAGE)`; all 40 tests pass.

### What the slice implements

- **Owner on placement.** The key panel item gets an `ItemPart::on_block_placed_by`; the placer's
  UUID is saved as `owner` (32 hex digits) on the keypad's block entity. A dispenser place has no
  placer and leaves the keypad unowned, as upstream's `IOwnable` has no owner there either.
- **Install.** `BlockPart::use_item_on`/`use_without_item` check the held stack before the block's
  own use (upstream `ModuleItem.useOn` runs before the keypad's use): the owner's harming module is
  added to the saved `modules` list once, and one item leaves the hand through
  `ItemPlace::PlayerHand::extract` (a creative hand keeps it, as upstream's `!isCreative()` guard).
- **Damage.** On a wrong submission `IPasscodeProtected`'s rule runs: `entity_hurt` with
  `securitycraft:incorrect_passcode` and 4 health points against the player who submitted, only
  when the module is installed. A right code and an empty/too-short entry hurt nothing.
- **Drop.** `Hooks::block_drops` adds the module item back when a keypad breaks, reading the saved
  data from the removed block entity (`IModuleInventory.dropAllModules`). *(Superseded by the
  PR #1544 review fix: a level-side removal never reaches `block_drops`, so the drop now runs from
  `block_changed` + `level_tick_post`; see the last section.)*
- **Content.** `securitycraft:harming_module` (stack 1, the jar's model and texture), its shaped
  recipe, the `securitycraft:incorrect_passcode` damage type, the death-message lang lines, and the
  creative-tab entry.
- **Readout.** `/securitycraft info` gains `modules=none|harming` and `owner=set|none` so the
  harness (and a server operator) can read the state; both are harness additions, not gameplay.

### Verification (SDK `0a875da`, 2026-11-21)

Every step ran in a `ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; the SDK steps ran
locally (`BUILD_LOCAL=1`, the remote box has no `cargo-horizon-mod`), on rustc 1.97.0. Logs are in
[`../_shared/evidence/securitycraft/`](../../_shared/evidence/securitycraft/).

| Check | Command | Result |
|---|---|---|
| Tests | `cargo test --offline` | **40 passed**, 0 failed (`sdk-0a875da-test.log`) |
| Formatting | `cargo fmt --check` | clean |
| Lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`sdk-0a875da-clippy.log`) |
| Package ×2 | `rm -f Cargo.lock; cargo horizon-mod build` twice + `cmp` | **identical** (`sdk-0a875da-build.log`) |
| Delivered | `securitycraft.hmod` | **52,759 B**, SHA-256 `3497bfcca16344bb6d6f4dd03b48ae6e125aabfcbb1d7d87023a0ab445681d11`, **47 entries**, **no `Cargo.lock`** |
| Registry check | `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, **EXIT 0** (`sdk-0a875da-check.log`) |
| Pinned build | `cargo horizon-mod fits securitycraft.hmod --build 0a875da` | `0a875da  made for`, EXIT 0, artefact SHA quoted in `sdk-fits.log` |
| Another build | `cargo horizon-mod fits securitycraft.hmod --build 0b14f8a` | `0b14f8a  fits`, EXIT 0, same artefact SHA in `sdk-fits-other.log` |
| Gate | `python3 ports/tools/verify-sdk-package.py securitycraft` | **1/1** |
| In game PASS | launcher-built game carrying the package | **PASS + `EXIT:0`** (`securitycraft-e2e/e2e-launcher.log`) |
| Control | the same scenario on the unmodified Nightly (`GAME=$CONTROL_GAME`) | **`EXIT:1`** at phase 1, the mod's ids unknown (`e2e-control.log`) |
| SELFTEST (door) | `SELFTEST=1` | **`EXIT:1`** (`e2e-selftest.log`) |
| SELFTEST (module) | `SELFTEST=harming` | **`EXIT:1`** at phase 12, the differential expected the missing damage (`e2e-selftest-harming.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=5 ok=5`, exit 0 |

Transcript quoted from `e2e-launcher.log`:

```
recipe result: securitycraft:harming_module
grid slots still holding an item: 0
fresh module keypad: securitycraft keypad 16 101 9 code=none entered= mode=check signal=0 powered=false status=0 modules=none owner=set door=none
after installing the module: securitycraft keypad 16 101 9 code=set entered= mode=check signal=0 powered=false status=2 modules=harming owner=set door=none
module still carried after a creative install: 1
wrong code with the harming module: … status=1 modules=harming owner=set door=none health 20.0 -> 16.0
right code with the harming module: … status=3 modules=harming owner=set door=none health 16.83333396911621 -> 18.666667938232425
break: {"block":"minecraft:air","drops":{"securitycraft:harming_module":1,"securitycraft:keypad_item":1}}
wrong code without the module: … status=1 modules=none owner=set door=none health 19.333332061767575 -> 19.333332061767575
PASS: …; the harming module recipe crafted from seven iron, paper and an arrow and consumed them; the placer became the keypad's owner, the owner installed the module, a wrong code took two hearts and a right code none, and the module dropped when the keypad broke; a keypad without the module did not hurt a wrong code
EXIT:0
```

The two-hearts value is read on the tick after the submit; reading it after the next `/securitycraft
info` (six more ticks) shows 16.83 because full-food regeneration has already healed some of it —
the first run of this phase caught that and the read was moved before the extra wait.

### Coverage of this slice

| Family | Status |
|---|---|
| `securitycraft:harming_module` item (give, read back, install) | **tested** — phase 10 (`modules=harming`) |
| harming module recipe (III/IPI/ISI) | **tested** — phase 9: output id/count and an emptied grid (ingredients consumed) |
| wrong-code damage with the module | **tested** — phase 10: 20.0 → 16.0 (two hearts) |
| right code with the module | **tested** — phase 10: no loss |
| keypad without the module (differential) | **tested** — phase 12: no loss |
| module drop on break | **tested** — phase 11: `drops` names the module |
| owner recorded on placement | **tested** — phase 10: `owner=set` |
| non-owner install refused | **unit test** `only_the_keypad_owner_may_install_a_module`; not in-game (the harness has one player) |
| allowlist / denylist / smart / disguise modules | **not ported** — the next module slices |
| customize screen (remove/toggle a module) | **not ported** — no module inventory screen yet |
| container closes after a harming hit | **not ported** — upstream's `player.closeContainer()`; the screen stays open here |
| smart-module cooldown | **deviation kept** — the port's throttle is unconditional since review #1197, so a keypad without the smart module still rate-limits |

### Deviations and gaps recorded by this slice

1. **Modules are stored as ids, not item stacks.** Upstream saves an `ItemStack` list; the port
   saves `modules` as a list of item-id strings. Installing, the effect and the drop are faithful;
   a customize screen that round-trips components would need the stacks.
2. **Only the harming module's key is resolved for the drop**, at registration
   (`HARMING_MODULE_ITEM`): a `block_drops` listener has a `LevelHost`, not a `Host`, so it cannot
   resolve an id at break time. A later module adds its key the same way.
3. **The recipe's ingredients are vanilla ids.** Upstream keys the iron on `c:ingots/iron` and the
   arrow on `minecraft:arrows`; the pack's convention tags are not shipped here, so the carry uses
   `minecraft:iron_ingot` and `minecraft:arrow`, the same items the tags resolve to in ATM10.
4. **Who may set the first code is still open.** The owner is now saved, but the set screen and
   `/securitycraft code` do not yet check it (upstream's `verifyPasscodeSet` refuses a stranger).
   That belongs to the owner/allowlist slice.
5. **Creative is invulnerable**, so the damage phase switches to survival; the module install phase
   stays in creative to prove the `!isCreative()` keep-the-stack rule.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

## Review fixes (2026-11-21, PR #1544) — 0.2.3

Three findings on 0.2.2 (`levzzz5154`): ship the `Cargo.lock` the launcher's clean build needs,
preserve the installed module when the removal is not a loot roll, and keep the upstream MIT
copyright line verbatim. Each was reproduced red before the change; the reds and their logs are
quoted below.

### 1. The shipped package carries its `Cargo.lock`

**Red (the archive exactly as published, 0.2.2).** The package was extracted into a fresh
directory (no `target/`, no build tree) and given to the launcher's clean build:

```
$ ports/_shared/bin/launcher-0.2.92/horizon-launcher native-build \
      ~/.cache/horizon-e2e/red-lock/securitycraft-0.2.2 --instance e2e --trust-native-code --json
Checking 1 mod
Using the mod tools for the Nov 20 update kept from an earlier build
Prebuilt game: only the mods compile (optimized)
Compiling the mods against the prebuilt game
error: /home/adlf/.cache/horizon-e2e/root/data/horizon-launcher/native/mods.incoming/securitycraft/Cargo.lock: No such file or directory (os error 2)
NATIVE_BUILD_EXIT:1
```

Log: `../_shared/evidence/securitycraft/review-1544-red-lock.log`. The same pair was then run on
the **final** archive so the receipt belongs to the bytes that ship: with the lock removed from the
extracted 0.2.3, the identical error (`review-1544-red-lock-final.log`).

**Fix.** `Cargo.lock` (49 packages, sha256 `8d448753f648…b296315`) is packed in the archive and
the build step no longer runs `rm -f Cargo.lock` for this port. The archive now has **48 entries**
(the 47 content/code files + `Cargo.lock`).

**This is a deliberate, package-scoped exception** to the campaign's usual lock-free package rule
(ARCHITECTURE.md §5, HANDOVER §0al). The rule exists because `cargo test` writes a lock that rides
into a content-only package where nothing needs it. This package **declares a registry crate**
(`sha2 = "0.10"`, the salted digest in `src/runtime.rs`), so a package without the lock cannot be
built by the launcher at all — the reviewer's finding. The gate now agrees:
`verify-sdk-package.py` prints `ok  packed Cargo.lock (declares a dependency)  48 entries; the
launcher's clean build needs it`, and still refuses a lock in a package that declares no dependency.
The exception is written in `Cargo.toml` beside the dependency as well.

**Green (clean cache + receipt).** A fresh extraction of the final archive, no build tree:

```
$ ports/_shared/bin/launcher-0.2.92/horizon-launcher native-build \
      ~/.cache/horizon-e2e/red-lock/securitycraft-0.2.3-final --instance e2e --trust-native-code --json
Checking 1 mod
Prebuilt game: only the mods compile (optimized)
Compiling the mods against the prebuilt game
Native mod adds 9 registry crates
Registry dependencies are native code. Their build scripts and proc macros run during compilation, before the game starts.
Build scripts: generic-array 0.14.7
Installing the build
Your mods are built (took 6 s).
native-0a875da-6744658cf70e built and selected for its instance.
NATIVE_BUILD_EXIT:0
```

The receipt is [`../_shared/evidence/securitycraft/review-1544-lock-receipt.md`](../../_shared/evidence/securitycraft/review-1544-lock-receipt.md)
(archive sha, launcher version, both logs); the nine
registry crates (`sha2`, `digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum`,
`cpufeatures`, `cfg-if`, `libc`) are exactly the graph the lock pins. The launcher's own downloaded
crate cache is shared, as for every other port; what is fresh is the package directory and the
whole build of the mod against the prebuilt game — there is no incremental tree and no lock the
machine could have carried over.

### 2. The installed module survives a removal that is not a loot roll

**Red (unit, rule absent).** The rule was stubbed to return nothing and the test run first:

```
---- support::tests::the_modules_drop_from_the_saved_list_without_a_loot_roll stdout ----

thread 'support::tests::the_modules_drop_from_the_saved_list_without_a_loot_roll' (2602192) panicked at src/support.rs:1153:9:
assertion `left == right` failed: an installed module is dropped by the removal itself
  left: []
 right: ["securitycraft:harming_module"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

test result: FAILED. 40 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Log: `../_shared/evidence/securitycraft/review-1544-red-unit.log`.

**Red (in game, 0.2.2).** The new phase 11b installed the module, removed the keypad with
`/setblock … minecraft:air` — no loot roll, no player break — and asked the world for item
entities:

```
== phase 11b: a level-side removal leaves the installed module in the world ==
keypad before the level-side removal: securitycraft keypad 18 101 -1 code=set entered= mode=check signal=0 powered=false status=2 modules=harming owner=set door=none
after setblock: minecraft:air
items near the removed keypad: []
FAIL: the installed module vanished with a level-side removal (no item entity)
EXIT:1
```

Log: `../_shared/evidence/securitycraft/review-1544-red-nonloot.log`.

**Root cause, measured.** `block_drops` is a *loot roll*: it runs for a player's or mob's break and
for an explosion's merged loot, but not for a level-side removal, so 0.2.2's module drop never ran
for `/setblock`. The obvious replacement, `pre_remove_side_effects`, was probed on both paths
(`review-1544-probe-nonloot.log`): it **does** run for a player's break (the module was dropped and
picked up — `break: … "picked_up":{"securitycraft:harming_module":1}`), and it **does not** run for
`/setblock` (`items: []`, the module gone). The entity is also already out of the world at
`block_changed` time, so the module list cannot be read there either.

**Fix: keep the list while the keypad stands, drop it when the block is gone.**

- `src/support.rs` keeps the installed-module list per `(dimension id, pos)` (`INSTALLED`),
  refreshed when the module is installed (`on_use`) and on every keypad tick, and forgotten when a
  new block is placed at the position.
- `Hooks::block_changed` (every state commit — player break, `/setblock`, `/fill`, a piston pushing
  a block in, an explosion) sees `old = keypad, new = not keypad`, takes that list and queues it in
  `PENDING_DROPS`.
- `Hooks::level_tick_post` (read-only `block_changed` cannot spawn) spawns each module with
  `LevelRef::spawn_item` at the block's centre with vanilla's default pickup delay of 10, then
  forgets the entry.
- The old `block_drops` module code is removed, so a player's break drops the module exactly once.
  This is upstream's `IModuleInventory.dropAllModules` reached from a hook every removal path
  reaches.

**Green.** Phase 11 (player break) still drops it, and phase 11b (level-side) now leaves it in the
world:

```
break: {"block":"minecraft:air","drops":{"securitycraft:harming_module":1,"securitycraft:keypad_item":1}}
keypad before the level-side removal: securitycraft keypad 22 101 -4 code=set entered= mode=check signal=0 powered=false status=2 modules=harming owner=set door=none
after setblock: minecraft:air
items near the removed keypad: [{"item":"securitycraft:harming_module","count":1,"pos":[22.98380249450912,101.74249315365324,-3.205940992901731]}]
```

Standalone probe on both paths after the fix (`review-1544-probe-nonloot.log`):
`/setblock` → `items: [{"item":"securitycraft:harming_module","count":1,…}]`; a survival break →
`drops: {"securitycraft:harming_module":1}`, `picked_up: {}`.

**Engine behaviour worth recording upstream (HANDOVER §9l candidate).** A level-side write
(`/setblock`, `/fill`) removes the block entity **without** calling
`BlockEntity.preRemoveSideEffects` at this pin, and `block_changed` is the only notice it gives —
with a read-only level and the entity already gone. A mod that must preserve block-entity content
on *every* removal therefore has to maintain the content itself (here: a tick-refreshed stash) and
spawn it at `level_tick_post`. `block_drops` is not a universal removal hook and
`pre_remove_side_effects` is not either; the two together still miss this path.

### 3. The upstream MIT copyright line is kept verbatim

**Red.**

```
$ grep -F "Copyright (c) 2025 The SecurityCraft development team" LICENSE
grep exit: 1
$ head -4 LICENSE
MIT License

Copyright (c) Geforce and contributors (SecurityCraft)
Copyright (c) 2026 AdLf0208 (Horizon port)
$ curl -sS https://raw.githubusercontent.com/Geforce132/SecurityCraft/c4d35b0c8bf85c2a94c37a1f8ce37ef5ce1a6aa8/LICENSE | head -4
MIT License

Copyright (c) 2025 The SecurityCraft development team
```

Log: `../_shared/evidence/securitycraft/review-1544-red-licence.log`. The packaged `LICENSE` had
the same paraphrase, so an artifact-only download did not carry the original line.

**Fix.** `LICENSE` now reads `Copyright (c) 2025 The SecurityCraft development team` verbatim, above
the port's own `Copyright (c) 2026 AdLf0208 (Horizon port)`, and `LICENSE-NOTICE.md` quotes the same
line and the licence URL.

**Upstream author and URL for the listing.** The jar's `META-INF/neoforge.mods.toml` says
`authors="Geforce, bl4ckscor3, Redstone_Dubstep, and ChainmailPickaxe"` and
`license="MIT https://github.com/Geforce132/SecurityCraft/blob/master/LICENSE"`;
`horizon-mod.toml` now carries that author line exactly (beside `AdLf0208` for the port), so the
registry listing names the real authors. Repository: <https://github.com/Geforce132/SecurityCraft>
(`1.21.1` branch, commit `c4d35b0c8bf85c2a94c37a1f8ce37ef5ce1a6aa8`).

### Verification (0.2.3, SDK `0a875da`, 2026-11-21)

Every step ran in a `build-slot.sh` lease with `CARGO_INCREMENTAL=0`; logs are in
[`../_shared/evidence/securitycraft/`](../../_shared/evidence/securitycraft/). The unit-test loop
used `sha2` from the packed lock's pins.

| Check | Command | Result |
|---|---|---|
| Tests | `cargo test --offline` | **41 passed**, 0 failed (`review-1544-test.log`) |
| Formatting | `cargo fmt --check` | exit 0 (`review-1544-fmt.log`) |
| Lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`review-1544-clippy.log`) |
| Package ×2 | `cargo horizon-mod build` twice + `cmp` | **identical** (`review-1544-build-1.log`, `-2.log`) |
| Delivered | `securitycraft.hmod` | **57,881 B**, SHA-256 `978a23652d2580b7e0486ff0d7f819c20330593fff3ab140edde5ef1f19e7112`, **48 entries** (incl. `Cargo.lock`) |
| Registry check | `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.3 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, **EXIT 0** (`review-1544-check.log`) |
| Pinned build | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, EXIT 0, sha `978a2365…` (`review-1544-fits.log`) |
| Another build | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, EXIT 0, sha `978a2365…` (`review-1544-fits-other.log`) |
| Clean launcher build | extracted archive, `native-build` | `native-0a875da-6744658cf70e built`, EXIT 0 (`review-1544-lock-clean-build.log`) |
| Clean build, lock removed | same extraction | `error: …Cargo.lock: No such file or directory (os error 2)`, EXIT 1 (`review-1544-red-lock-final.log`) |
| Gate | `verify-sdk-package.py securitycraft` | **1/1**, incl. `packed Cargo.lock (declares a dependency)` (`review-1544-verify-sdk-package.log`) |
| In game PASS | launcher-built game carrying 0.2.3 | **PASS + `EXIT:0`** (`securitycraft-e2e/e2e-launcher.log`) |
| Control | the same scenario on the unmodified Nightly | **`EXIT:1`** at phase 1 (the mod's ids are unknown) (`e2e-launcher-control.log`) |
| SELFTEST (door) | `SELFTEST=1` | **`EXIT:1`**, "expected the door OPEN after a wrong code" (`e2e-selftest.log`) |
| SELFTEST (module) | `SELFTEST=harming` | **`EXIT:1`**, "expected a keypad without the module to hurt the player" (`e2e-selftest-harming.log`) |
| SELFTEST (level removal) | `SELFTEST=nonloot` | **`EXIT:1`**, "expected the module to vanish with a level-side removal" — the phase 11b assertion fails when inverted, on the delivered build (`e2e-selftest-nonloot.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=6 ok=6`, exit 0 |

The control leg now says why it fails (`FAIL: the game refused securitycraft:iron_door_reinforced:
the package is not loaded`) instead of `set -e` aborting silently. Phase 11 also waits for `give`'s
next tick before selecting the pickaxe; a loaded machine made the old ordering race, and the leg
now reports that rather than exiting without a verdict.

`src/runtime.rs` is unchanged by this review (`faafca149260…`), and it is byte-identical inside the
package. `src/support.rs` (`c917494b…`) carries the stash and the two hooks; `src/lib.rs`
(`b536ea84…`) names them.

### Changelog for the resubmission (0.2.3)

> Supersedes #1544. Three review findings fixed: (1) the package now **ships its `Cargo.lock`** —
> it declares `sha2`, and the launcher's clean `native-build` refuses a package that declares a
> registry crate without a lock (`error: …/Cargo.lock: No such file or directory (os error 2)`); the
> same extraction with the lock builds (`native-0a875da-6744658cf70e built`, receipt in
> `../_shared/evidence/securitycraft/review-1544-lock-clean-build.log`). This is a package-scoped
> exception to the lock-free rule, recorded in `Cargo.toml` and here. (2) An installed module now
> survives a removal that is not a loot roll: the drop moved from the loot-only `block_drops` to a
> list kept while the keypad stands (`block_changed` + `level_tick_post`), proven in game with a
> level-side `/setblock` and inverted in `SELFTEST=nonloot`. (3) `LICENSE` keeps upstream's line
> verbatim — `Copyright (c) 2025 The SecurityCraft development team` — and `horizon-mod.toml` names
> the jar's authors; the port's own copyright follows. `src/runtime.rs` is unchanged.

### Coverage after this review

| Family | Status |
|---|---|
| module drop on a player's break | **tested** — phase 11 (`drops` names the module; `picked_up` accepted) |
| module drop on a level-side removal (`/setblock`) | **tested** — phase 11b: an item entity at the keypad's position |
| one removal drops one module (no double drop) | **unit test** `drop_from_the_saved_list…` takes the list once; phase 11's `drops` count is 1 |
| the archive builds on the launcher's clean path | **tested** — `review-1544-lock-clean-build.log`, and the counterfactual without the lock |
| the upstream copyright line travels in the package | **tested** — `unzip -p securitycraft.hmod LICENSE` (red log shows the old paraphrase) |

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

## Slice 4 — the owner gate on setting a code (2026-11-22, SDK `0a875da`) — 0.2.4

**Slice.** The gap line is this port's own, slice-3 deviation 4:

> **Who may set the first code is still open.** The owner is now saved, but the set screen and
> `/securitycraft code` do not yet check it (upstream's `verifyPasscodeSet` refuses a stranger).

This slice closes exactly that: one pure rule, the set path and the harness control. Class
**content**, tier **deep**; the delivered shape is the port's existing **logic + machines** half
(block entity, menu, screen, redstone) and it adds **no content, no renderer, no new block** — the
rule and its two client/server call sites. Nothing else in the gap list changed.

### Upstream read (this job, `Geforce132/SecurityCraft`, branch `1.21.1`, commit `c4d35b0c`)

Behaviour only, from the pinned commit's files:

| File | What it says |
|---|---|
| `api/IPasscodeProtected.java` | `verifyPasscodeSet`: a passcode already set → `true`; no passcode and `ownable.isOwnedBy(player)` → open the **set** screen; otherwise the player gets `messages.securitycraft:passcodeProtected.notSetUp` and **no screen**, and the method returns `false`. |
| `blocks/KeypadBlock.java` (`useWithoutItem`) | powered → `PASS`; else `verifyPasscodeSet(...)` — only when it returns `true` does the deny/allow/check-screen branch run, so a stranger's click opens nothing. |
| `api/IOwnable.java` | `isOwnedBy(Player)` → `getOwner().isTreatedTheSameAs(new Owner(player))`, so an unowned block (no owner saved) matches nobody. |
| `network/server/SetPasscode.java` (`handle`) | the server refuses the hashed code unless `!player.isSpectator() && (!(pp instanceof IOwnable ownable) || ownable.isOwnedBy(player))` — ownership is checked **again** at the actual set, not only when the screen opens. |

### Red proof (TDD, before the rule existed)

`src/runtime.rs` gained `a_stranger_may_not_set_the_first_code` first, with `may_set_code`
stubbed to `true` (the current behaviour). In a slot:

```
---- runtime::tests::a_stranger_may_not_set_the_first_code stdout ----

thread 'runtime::tests::a_stranger_may_not_set_the_first_code' (1531478) panicked at src/runtime.rs:703:9:
a stranger sets nothing, as upstream's verifyPasscodeSet refuses it
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

test result: FAILED. 41 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Log: [`../_shared/evidence/securitycraft/slice4-red.log`](../../_shared/evidence/securitycraft/slice4-red.log).
`may_set_code(owner, player)` then became `owner == Some(player)`; **42 passed**, 0 failed.

### What the slice implements

- **The rule.** `runtime::may_set_code(owner: Option<u128>, player: u128) -> bool` — upstream's
  `IOwnable.isOwnedBy`, documented with both places upstream applies it (the set-screen entry and
  the server-side set). It is the same predicate as `may_install` because upstream uses one
  ownable check for both privileges; the two names stay apart because the privileges are.
- **The set-screen entry** (`support::on_use`). A keypad with no code now opens `Mode::Set` only
  for its owner; anyone else gets the click cancelled (`InteractionResult::Fail`, the same answer a
  powered keypad gives — the content block opens the menu itself, so the refusal has to end the
  click) and `status=7` in the readout. That is upstream's "not set up" answer without the chat
  message the drawn panel cannot send.
- **The set submit** (`support::apply_key`). `Mode::Set` + a non-owner is refused before any salt
  or digest is written. Upstream never reaches this state (a stranger never gets the screen), but
  the port's screen can outlive an ownership change and a menu button is client-sent, so the server
  checks too — the same second check `SetPasscode.handle` makes. The refusal is `status=1`
  ("Wrong code": the panel has one failure line) and stores nothing.
- **The harness control** (`/securitycraft code`) got the same gate: only the asking player, when
  they own the keypad, may set a code — it is a set, so it obeys `SetPasscode.handle`'s rule. A
  refusal answers `Only the keypad's owner can set its code.`
- **The counterfactual command** (`/securitycraft owner <pos> <self|stranger|32-hex>`, op level 2).
  A single-player harness cannot drive a second player, so this changes the owner: `stranger`
  writes the fixed harness UUID `deadbeefdeadbeefdeadbeefdeadbeef`, `self` the acting player's.
  It changes the owner **only** — it does not reproduce the Universal Owner Changer's code reset,
  because that block is not ported and the "stranger at a keypad that has a code" state is what
  the harness needs (recorded as a deviation).

### Verification (0.2.4, SDK `0a875da`, 2026-11-22)

Every step ran in a `ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; logs in
[`../_shared/evidence/securitycraft/`](../../_shared/evidence/securitycraft/). The launcher's
native build runs one at a time machine-wide, so the game builds waited for the launcher lock.

| Check | Command | Result |
|---|---|---|
| Red | `cargo test --offline` with the rule stubbed | **1 failed / 41 passed** (`slice4-red.log`) |
| Tests | `cargo test --offline` | **42 passed**, 0 failed (`slice4-test.log`) |
| Formatting | `cargo fmt --check` | exit 0 |
| Lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`slice4-clippy.log`) |
| Package ×2 | `cargo horizon-mod build` twice + `cmp` | **identical** (`slice4-build-1.log`, `-2.log`); the lock is **kept** (declared `sha2`) |
| Delivered | `securitycraft.hmod` | **59,524 B**, SHA-256 `eb42fd50fa0b81da38a083b2dd0c6b25847133d4aa8705469f15992b07679b91`, **48 entries** (incl. `Cargo.lock`) |
| Registry check | `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.4 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, **EXIT 0** (`slice4-check.log`) |
| Pinned build | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, EXIT 0 (`slice4-fits.log`) |
| Another build | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, EXIT 0, same artifact SHA (`slice4-fits-other.log`) |
| Clean build of the archive | extracted 0.2.4 → `horizon-launcher native-build` | `Compiling securitycraft v0.2.4`, **`native-0a875da-e141cb80db51 built`**, EXIT 0 (`slice4-clean-build.log`) |
| Gate | `python3 ports/tools/verify-sdk-package.py securitycraft` | **1/1** (`slice4-verify-sdk-package.log`) |
| In game PASS | launcher-built game carrying 0.2.4 | **PASS + `EXIT:0`** (`securitycraft-e2e/e2e-launcher.log`) |
| Control | the same scenario on the unmodified Nightly (`GAME=$CONTROL_GAME`) | **`EXIT:1`** at phase 1 (`e2e-control.log`) |
| SELFTEST (door) | `SELFTEST=1` | **`EXIT:1`**, "expected the door OPEN after a wrong code" (`e2e-selftest.log`) |
| SELFTEST (owner) | `SELFTEST=owner` | **`EXIT:1`**, "expected a stranger's code to be stored" (`e2e-selftest-owner.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=7 ok=7`, exit 0 (`slice4-check-e2e-logs.log`) |

Phase 13 quoted from `e2e-launcher.log`:

```
owner keypad: securitycraft keypad 18 101 4 code=none entered= mode=check signal=0 powered=false status=0 modules=none owner=set door=none
a stranger's right click opened 'playing': securitycraft keypad 18 101 4 code=none entered= mode=check signal=0 powered=false status=7 modules=none owner=set door=none
the owner's set screen: securitycraft keypad 18 101 4 code=none entered= mode=set signal=0 powered=false status=0 modules=none owner=set door=none
after a stranger's set submit: securitycraft keypad 18 101 4 code=none entered= mode=set signal=0 powered=false status=1 modules=none owner=set door=none
after the owner's set submit: securitycraft keypad 18 101 4 code=set entered= mode=check signal=0 powered=false status=2 modules=none owner=set door=none
a stranger's code command answered: Only the keypad's owner can set its code.
after a stranger's code command: securitycraft keypad 18 101 4 code=set entered= mode=check signal=0 powered=false status=2 modules=none owner=set door=none
the owner's code after the refused set: securitycraft keypad 18 101 4 code=set entered= mode=check signal=58 powered=true status=3 modules=none owner=set door=none
PASS: …and only the owner may set a keypad's first code — a stranger's click opened no set screen, a set screen that outlived an owner change stored nothing, the owner set it, and a stranger's /securitycraft code was refused
EXIT:0
```

`src/runtime.rs` (`a0fca2d0…`), `src/support.rs` (`d0db7332…`) and `src/lib.rs` (`45f41e01…`)
are byte-identical inside the package (`slice4-packed-src.log`); the rule is the only change to
`runtime.rs`, the three call sites and the owner command are the glue.

The counterfactual is the only way a single-player harness can stand a stranger in front of the
keypad: `/securitycraft owner … stranger` moves the owner, and each leg's assertion would pass
vacuously if the command did nothing (`may_set_code` would then find the acting player the owner
and store the code), so the legs are not self-satisfying. `SELFTEST=owner` inverts the refused-set
assertion on the delivered build and fails exactly there.

### Coverage after this slice

| Family | Status |
|---|---|
| a stranger's right click on a code-less keypad | **tested** — phase 13a: `screen=playing`, `status=7`, `code=none` |
| a set submit from a screen that outlived an owner change | **tested** — phase 13b: `status=1`, `code=none` |
| the owner sets the first code | **tested** — phase 13c: `code=set`, `mode=check` |
| `/securitycraft code` by a stranger | **tested** — phase 13d: refused with `Only the keypad's owner…`, the owner's code still opens the keypad (`powered=true`) |
| the rule itself (owner / stranger / unowned) | **unit test** `a_stranger_may_not_set_the_first_code` |
| allowlist / denylist modules, and their "allowed → open without a code" branch | **not ported** — the next slice |
| the disguise / smart modules, the `disabled` option, waterlogging, ceiling/floor mounting, the passcode chest/barrel/furnace/smoker, keypad door and trapdoor, the reinforced-block family, cameras, mines, lasers, scanners, the alarm, display cases, the frame, the door's in-world recipe, the owner-only breaking speed, the config file | **not ported** — unchanged from slice 1/3 (see the gap list above) |

### Deviations recorded by this slice

1. **A refused set reads `status=1` ("Wrong code").** Upstream never shows a stranger the set
   screen, so it never needs a refusal status; the port's screen can outlive the entry check, and
   the drawn panel has one failure line. No code is written, and `/securitycraft info` reports the
   truth (`code=none`).
2. **`/securitycraft owner` is harness-only and changes the owner alone.** It stands in for a
   second player; upstream's Owner Changer also resets the passcode (`onOwnerChanged`), which the
   harness deliberately does not do so that "a stranger at a keypad that has a code" is reachable.
3. **An unowned keypad is nobody's to set.** A keypad placed by a dispenser, or saved by a build
   before slice 3, has no owner and can no longer have a code set — faithful to `isOwnedBy` on an
   ownerless block, and a behaviour change for such saves (break and replace it with a placer).
4. **No "not set up" chat line** for the stranger: the port's panel is drawn by the mod and the
   refusal is `status=7` in the readout rather than a `messages.securitycraft` line (the mod's
   message keys are not carried).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

## R9 — the hash is the port's own code (2026-11-23, SDK `0a875da`) — 0.2.5

**Why this slice.** The registry refuses any package whose `Cargo.toml` declares a dependency other
than the SDK (`tools/native-subset/src/manifests.rs`, R9; `var/autopilot/PLAN.md` "R9", read
2026-11-23). This package declared `sha2 = "0.10"` for the keypad's salted digest and packed the
resolved `Cargo.lock`; #1800 (actuallyadditions) was refused at `495701cf` for `serde_json`, so the
rule is enforced, not advisory. The job is a **dependency removal, not a feature**: the keypad,
the reinforced door, the harming module and the owner gate behave exactly as in 0.2.4. Class
**content**, tier **deep** — the delivered shape does not change.

**The decision: implement SHA-256 in `src/runtime.rs`, not drop the digest.** The alternatives were
a weaker hash (a plaintext-equivalent secret in the save) or a new id/kind (impossible: this entry is
published). A hash is 60 lines of well-specified arithmetic, so the port carries its own: `const
SHA256_K` (the 64 round constants), `SHA256_INITIAL`, `sha256` (padding and block loop) and
`sha256_compress` (message schedule and 64 rounds), written from FIPS 180-4, private to the rules
module. No `unsafe` (R15), no `extern` (R16), no macros (R17) — plain functions over `[u8]`/`[u32]`.
`hash_code` now feeds `salt ‖ digits` to it, so the saved bytes (a hex digest plus the salt) and the
NBT shape are unchanged: an existing keypad's code still opens it, which the relaunch leg proves.

**The policy is dead.** "Declare a registry dependency and pack its lock" was right for the
launcher's clean build (0.2.3's review asked for exactly that) and is **dead for publishing**: R9
refuses the dependency, and R10 accepts a lock only as the previous release's. An SDK-only package
ships none. `Cargo.toml` now ends at the SDK path dependency — no `[workspace]` either (R9 refuses
it even empty).

### Red (TDD, before the primitive existed)

`sha2` was removed from `Cargo.toml`, `hash_code` was pointed at a `sha256` stub returning
`[0u8; 32]`, and the new padding vectors were added. In a build slot:

```
---- runtime::tests::the_digest_is_sha256_of_the_salt_then_the_code stdout ----

thread 'runtime::tests::the_digest_is_sha256_of_the_salt_then_the_code' (3262368) panicked at src/runtime.rs:475:9:
assertion `left == right` failed
  left: Some("0000000000000000000000000000000000000000000000000000000000000000")
 right: Some("bad0b4f7cae08eb7c1b5acc763a8ed25242e2c9173e81f3f624b1fa5303cf6ce")

---- runtime::tests::the_hash_matches_the_standard_vectors stdout ----

thread 'runtime::tests::the_hash_matches_the_standard_vectors' (3262370) panicked at src/runtime.rs:486:9:
assertion `left == right` failed: the empty message
  left: "0000000000000000000000000000000000000000000000000000000000000000"
 right: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

test result: FAILED. 36 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Log: [`../_shared/evidence/securitycraft/r9-red.log`](../../_shared/evidence/securitycraft/r9-red.log).
After the primitive was written: **43 passed**, 0 failed (`r9-test.log`, `sdk-test.log`).

### Test vectors (the digest rule, in `runtime.rs`)

| Message | Expected SHA-256 |
|---|---|
| `bytes.fromhex("000102…0f") ‖ b"1234"` (the existing digest vector) | `bad0b4f7cae08eb7c1b5acc763a8ed25242e2c9173e81f3f624b1fa5303cf6ce` |
| the empty message | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 55 × `a` (one padded block: the `0x80` and the 8-byte length still fit) | `9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318` |
| 56 × `a` (the length no longer fits; a second block is forced) | `b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a` |
| 1000 × `a` (many blocks) | `41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3` |

The same code under `OTHER_SALT` keeps its vector (`3c98a0ee…cf43`), so the salt is still part of
the message, not of the state.

### Verification (0.2.5, SDK `0a875da`, 2026-11-23)

Every step ran in a `ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; the SDK and
launcher steps ran locally (`BUILD_LOCAL=1`, the remote box has no `cargo-horizon-mod`). Logs are in
[`../_shared/evidence/securitycraft/`](../../_shared/evidence/securitycraft/) and
[`../_shared/evidence/securitycraft-e2e/`](../../_shared/evidence/securitycraft-e2e/).

| Check | Command | Result |
|---|---|---|
| Red | `cargo test --offline` with the primitive stubbed | **36 passed / 7 failed** (`r9-red.log`) |
| Tests | `cargo test --offline` | **43 passed**, 0 failed (`r9-test.log`, `sdk-test.log`) |
| Formatting | `cargo fmt --check` | exit 0 |
| Lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`r9-clippy.log`) |
| Package ×2 | `rm -f Cargo.lock && cargo horizon-mod build` twice + `cmp` | **identical** (`r9-build-1.log`, `-2.log`) |
| Delivered | `securitycraft.hmod` | **59,040 B**, SHA-256 `7c93b68dc46f2ab00d533830c222795527abdb456942f8fc8fae84e72c074dc8`, **47 entries**, **no `Cargo.lock`** |
| Registry check | `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.5 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, **EXIT 0** (`r9-check.log`) |
| Pinned build | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, EXIT 0, `artifact checked: 59040 bytes, sha256 7c93b68d…` (`r9-fits.log`) |
| Another build | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, EXIT 0, same artifact SHA (`r9-fits-other.log`) |
| Launcher build | `launcher-game.sh build securitycraft` | `Compiling securitycraft (1 of 1)`; **no "Native mod adds N registry crates" line**; `native-0a875da-2be1c7ac3bc8 built` (`r9-launcher-build.log`) |
| Clean build of the archive | extracted 0.2.5 → `horizon-launcher native-build` | `Compiling securitycraft (1 of 1)`, `native-0a875da-7094b229ec0f built`, EXIT 0 (`r9-clean-build.log`) |
| Gate | `python3 ports/tools/verify-sdk-package.py securitycraft` | **1/1**, incl. `SDK-only dependencies (R9)`, `no packed Cargo.lock`, `package matches the tree` (`r9-verify-sdk-package.log`) |
| In game PASS | launcher-built game carrying 0.2.5 | **PASS + `EXIT:0`**, incl. phase 14 (`securitycraft-e2e/e2e-launcher.log`) |
| Control (differential) | the same scenario on the unmodified Nightly (`GAME=$CONTROL_GAME`) | **`EXIT:1`**, "the game refused securitycraft:iron_door_reinforced" (`e2e-control.log`) |
| SELFTEST (door) | `SELFTEST=1` | **`EXIT:1`** (`e2e-selftest.log`) |
| SELFTEST (owner) | `SELFTEST=owner` | **`EXIT:1`** at 13b (`e2e-selftest-owner.log`) |
| SELFTEST (relaunch) | `SELFTEST=relaunch` | **`EXIT:1`** at 14, "expected the relaunch to lose the stored digest" (`e2e-selftest-relaunch.log`) |
| SELFTEST (harming) | `SELFTEST=harming` | **`EXIT:1`** at 12 (`e2e-selftest-harming.log`) |
| SELFTEST (level removal) | `SELFTEST=nonloot` | **`EXIT:1`** at 11b (`e2e-selftest-nonloot.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=7 ok=7`, exit 0 |
| Recipe types | `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

### The digest survives a restart (new phase 14)

`tools/e2e.sh` now quits the game and relaunches the **same** world and game dir, then types the
code stored before the restart. Quoted from `e2e-launcher.log`:

```
== phase 14: a code set before a restart still opens the same keypad after it ==
keypad before the relaunch: securitycraft keypad 13 101 -6 code=set entered= mode=check signal=0 powered=false status=3 modules=none owner=set door=12,101,-6:closed
keypad after the relaunch: securitycraft keypad 13 101 -6 code=set entered= mode=check signal=0 powered=false status=3 modules=none owner=set door=12,101,-6:closed
a wrong code after the relaunch: securitycraft keypad 13 101 -6 code=set entered= mode=check signal=0 powered=false status=1 modules=none owner=set door=12,101,-6:closed
the right code after the relaunch: securitycraft keypad 13 101 -6 code=set entered= mode=check signal=58 powered=true status=3 modules=none owner=set door=12,101,-6:open
```

`SELFTEST=relaunch` inverts the "the digest survived" assertion and fails exactly there (log
above), so the leg is not self-satisfying.

### Coverage added by this slice

| Family | Status |
|---|---|
| the digest over one block (the salt + a 4-digit code) | **tested** — `the_digest_is_sha256_of_the_salt_then_the_code` |
| the empty message and the 55/56-byte padding boundary, and many blocks | **tested** — `the_hash_matches_the_standard_vectors` |
| a code set before a restart opens the same keypad after it | **tested** — phase 14 (PASS), inverted by `SELFTEST=relaunch`, `EXIT:1` |
| the package declares no dependency but the SDK and ships no lock | **tested** — `verify-sdk-package.py` (R9 rows), the launcher build prints no registry crates |

### One observed intermittent, recorded not hidden

On 2026-11-23 the `SELFTEST=owner` leg failed once at **phase 11b** (`items near the removed
keypad: []`, "the installed module vanished with a level-side removal"), before reaching its own
inverted assertion; the leg was green again on the re-run (`e2e-selftest-owner.log`), the PASS leg
was green twice, and a targeted probe of the same rig (10 installs, gaps of 0/1/2/4/8 ticks before
`/setblock`) kept the module **10 of 10** (`r9-probe-nonloot.log`). The failing run is kept as
[`../_shared/evidence/securitycraft/r9-owner-flake-11b.log`](../../_shared/evidence/securitycraft/r9-owner-flake-11b.log).
The digest change cannot reach that path (`hash_code` is called only from `submit`/`after_submit`),
so it is a **pre-existing** intermittent in the module stash (`block_changed` + `level_tick_post`,
0.2.3's design) and needs its own diagnosis slice; it is not silently called green. A candidate
explanation to test: a keypad tick that reads an empty module list races an install in the same
tick (`remember_modules` erases on an empty list), so the stash can be emptied just before the
removal.

### Changelog for the resubmission (0.2.5)

> Supersedes #1677 (0.2.4). The package now declares **no dependency but the SDK's
> `horizon-mod-api`** and ships **no `Cargo.lock`**: the registry refuses both (R9/R10, #1800 was
> closed for `serde_json`). The keypad's salted SHA-256 is the port's own implementation in
> `src/runtime.rs` (FIPS 180-4, no `unsafe`, no macros), with the empty-message and 55/56-byte
> padding vectors added beside the existing digest vector; behaviour, save format and the NBT keys
> are unchanged, and the e2e now includes a quit-and-relaunch leg proving a code stored before the
> restart still opens the keypad. `Cargo.toml` carries `[package]` and the SDK path dependency only.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

## Slice 5 — the allowlist module and "allowed → open without a code" (2026-11-24, SDK `0a875da`) — 0.2.6

**Slice.** The gap line is this port's slice-4 coverage table:

> allowlist / denylist modules, and their "allowed → open without a code" branch | not ported — the next slice

This slice takes the **allowlist module and that branch**. Class **content**, tier **deep** in the
ledger; the delivered shape is the port's existing **logic + machines** half (block entity `modules`
list, install via `use_item_on`, drawn panel, redstone) plus one carried module item with its recipe,
model, texture and lang — no new block, renderer or hook kind. The **denylist** is deliberately out
of scope: it is a second item (`blacklist_module`), recipe and list with its own `isDenied` branch;
with no denylist installed `isDenied` is always false, so the order in `useWithoutItem`
(`denied → allowed → code screen`) is exactly reproduced for a keypad carrying only an allowlist
module. Porting it would need a second carried item and list state and is left as the next gap.

### Upstream read (`Geforce132/SecurityCraft`, branch `1.21.1`, commit `c4d35b0c`)

Behaviour only (the clone was deleted after these findings were written); this remains a clean-room
Rust rewrite. Files read: `items/ModuleItem.java` (install by the owner, `acceptsModule`,
`!hasModule`, creative keeps the stack), `misc/ModuleType.java` (`ALLOWLIST`/`DENYLIST` map to the
jar's item ids `whitelist_module`/`blacklist_module`), `SCContent.java` (those registrations carry
`ListModuleData.EMPTY`), `components/ListModuleData.java` (`players`, `teams`, `affectEveryone`;
`MAX_PLAYERS = 50`; `isPlayerOnList` compares names case-insensitively), `api/IModuleInventory.java`
(`isAllowed`/`isDenied`), `blocks/KeypadBlock.java` (`useWithoutItem`, `activate`) and
`api/IPasscodeProtected.java` (`verifyPasscodeSet`, which returns false for an unset keypad).

**The exact rules, as read:**

- `isAllowed(entity)`: false without an ALLOWLIST module; with one, `affectEveryone` **or** the
  player's team on the team list **or** the player's name on the list (case-insensitive). An owner is
  looked up by `PlayerUtils.getNameFromPlayerOrMask`, everyone else by their display name.
- `isDenied(entity)`: false without a DENYLIST module; with one, `affectEveryone` denies everyone
  except the owner, otherwise the team/player lists deny. Deny is checked **before** allow.
- `useWithoutItem`: a powered keypad (`signalLength > 0`) does nothing; disabled → message; else
  `verifyPasscodeSet` (a set code passes; an unset one opens the owner's set screen and returns
  false); then denied → message only, allowed → message + **`activate`**, else the code screen.
  `activate` cycles `POWERED`, updates indirect neighbours and schedules the off tick.
- The jar's recipe `data/securitycraft/recipe/whitelist_module.json` is `III / IPI / ISI` with
  `I = c:ingots/iron`, `P = minecraft:paper`, `S = minecraft:paper`, one `securitycraft:whitelist_module`.
  The jar's lang names the item "Allowlist Module" and the keypad description says it "allow[s]
  listed players to use the block without knowing the code". The 16×16 texture was carried from the
  jar; the port's recipe uses `minecraft:iron_ingot` as in slice 3's harming recipe.

### Red proof (TDD, before the rule existed)

`src/runtime.rs` gained `an_allowed_player_activates_without_a_code` and
`the_allowlist_is_bounded_and_case_insensitive` first, with `ListData::with_player`/`contains` and
`is_allowed` stubbed to keep the old behaviour. A forwarded test step gave:

```
---- runtime::tests::an_allowed_player_activates_without_a_code stdout ----

thread 'runtime::tests::an_allowed_player_activates_without_a_code' (26422) panicked at src/runtime.rs:933:9:
a listed player is allowed without typing a code

---- runtime::tests::the_allowlist_is_bounded_and_case_insensitive stdout ----

thread 'runtime::tests::the_allowlist_is_bounded_and_case_insensitive' (26438) panicked at src/runtime.rs:958:9:
upstream compares player names case-insensitively

test result: FAILED. 43 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

Log: [`slice5-red.log`](../../_shared/evidence/securitycraft/slice5-red.log). The rules then became
`installed && (affects_everyone || contains(player))`, a 50-player, case-insensitive, duplicate-free
list; **47 passed**, 0 failed.

### What the slice implements

- **The rule** (`runtime::is_allowed`, `runtime::ListData`): the pure form of
  `IModuleInventory.isAllowed`, with the upstream maximum, case-insensitive names and the
  `affectEveryone` arm. No SDK types in `runtime.rs`, and the in-tree SHA-256 is untouched.
- **Install** (`support::on_use`): the held module may be `harming_module` **or** `allowlist_module`;
  the same owner gate, one-per-type and creative-keep-the-stack rules apply. The install list is
  saved and dropped by both removal paths (`block_changed` + `level_tick_post`) as before.
- **The branch** (`support::on_use`, after the `verifyPasscodeSet` equivalent): a coded keypad with
  the allowlist module and the acting player on its list powers itself for `DEFAULT_SIGNAL_TICKS`,
  sets `status=3` and ends the click — no key press, no code screen. A keypad with **no** code never
  reaches the branch, exactly as upstream's `verifyPasscodeSet` returns false; the owner still gets
  the set screen and a stranger is still refused.
- **Harness control** `/securitycraft allow <pos> <self|stranger|32-hex>` (op 2) **sets** the one
  entry of the installed allowlist; upstream's customize screen cannot be driven headlessly, and a
  single player must be able to stand for both sides. Only the owner may edit it.
- **Readout**: `/securitycraft info` gains `allow=<count>` and now prints both module ids
  (`modules=none|harming|allowlist|harming,allowlist`). These are harness additions, not gameplay.

### Verification (0.2.6, SDK `0a875da`, 2026-11-24)

Compile/test/format/check steps ran through **forwarded** `build-slot.sh` leases on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`, hostname quoted in
[`slice5-remote-probe.log`](../../_shared/evidence/securitycraft/slice5-remote-probe.log)) with
`CARGO_INCREMENTAL=0`; **no `BUILD_LOCAL=1`** was used. The game legs stayed local by design: the
remote box has no launcher or prepared resources and the e2e runs `--offscreen`. The launcher's
machine-wide native-build lock was held once by another job; the same step was retried and passed
(`Compiling securitycraft (1 of 1)`). Logs are under
[`../_shared/evidence/securitycraft/`](../../_shared/evidence/securitycraft/) and
[`../_shared/evidence/securitycraft-e2e/`](../../_shared/evidence/securitycraft-e2e/).

| Check | Command | Result |
|---|---|---|
| Red | forwarded `cargo test --offline` with the rule stubbed | **43 passed / 2 failed** (`slice5-red.log`) |
| Tests | forwarded `cargo test --offline` | **47 passed**, 0 failed (`slice5-test.log`, `sdk-test.log`) |
| Formatting | forwarded `cargo fmt --check` | exit 0 |
| Lints | forwarded `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`slice5-lint.log`) |
| Package ×2 | `rm -f Cargo.lock` before each; two builds in one lease + `cmp` | **byte-identical** (`slice5-build.log`) |
| Delivered | `securitycraft.hmod` | **64,114 B**, SHA-256 `8df73c3810f2ad10d1c473b5583eaf4752b6cff047081614b554fd3ca65d80a1`, **52 entries**, **no `Cargo.lock`** |
| Registry check | forwarded `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.6 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 (`slice5-check-fits.log`) |
| Pinned build | forwarded `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, `artifact checked: 64114 bytes, sha256 8df73c38…80a1` (`slice5-fits.log`) |
| Another build | forwarded `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, same SHA (`slice5-fits.log`) |
| Gate | `verify-sdk-package.py securitycraft` | **1/1**, incl. `package matches the tree`, SDK-only deps, no lock (`slice5-verify-sdk-package.log`) |
| Launcher build | `launcher-game.sh build securitycraft` | `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-59c49fbba148` (`slice5-launcher-build.log`) |
| In-game PASS | launcher-built game carrying 0.2.6 | **PASS + `EXIT:0`** (`securitycraft-e2e/e2e-launcher.log`) |
| Control | the scenario on the unmodified Nightly (`GAME=$CONTROL_GAME`) | **`EXIT:1`** at phase 1 (`e2e-control.log`) |
| Ids-unknown control | `CONTROL=1` | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| SELFTEST (door/harm/nonloot/owner/relaunch) | `SELFTEST=1\|harming\|nonloot\|owner\|relaunch` | **`EXIT:1`** at each own assertion |
| SELFTEST (allowlist) | `SELFTEST=allow` | **`EXIT:1`** at phase 14, "expected an allowed click not to power the keypad" (`e2e-selftest-allow.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=9 ok=9`, exit 0 (`slice5-check-e2e-logs.log`) |
| Recipe types | `check-recipe-types.py securitycraft` | `securitycraft: ok` |

Phase 14 quoted from `e2e-launcher.log`:

```
allowlist recipe result: securitycraft:allowlist_module
allowlist grid slots still holding an item: 0
coded allowlist keypad: securitycraft keypad 12 101 4 code=set entered= mode=check signal=0 powered=false status=2 modules=none allow=0 owner=set door=13,101,4:closed
after installing the allowlist module: securitycraft keypad 12 101 4 code=set entered= mode=check signal=0 powered=false status=2 modules=allowlist allow=0 owner=set door=13,101,4:closed
after adding the acting player: securitycraft keypad 12 101 4 code=set entered= mode=check signal=0 powered=false status=2 modules=allowlist allow=1 owner=set door=13,101,4:closed
allowed click with nothing typed: screen=playing securitycraft keypad 12 101 4 code=set entered= mode=check signal=52 powered=true status=3 modules=allowlist allow=1 owner=set door=13,101,4:open
non-listed click: screen=container securitycraft keypad 12 101 4 code=set entered= mode=check signal=0 powered=false status=0 modules=allowlist allow=1 owner=set door=13,101,4:closed
```

Both value pairs are fail-capable: `SELFTEST=allow` inverts the powered/open pair on the delivered
build and fails exactly there; the stranger differential proves the same click would otherwise open
the code screen. `src/runtime.rs` (`c2350624…`) is byte-identical inside the package.

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:allowlist_module` item (craft, give, install, drop) | **tested** — phase 14 recipe (id/count + emptied grid), install, `both_installed_modules_drop` |
| the allowlist recipe (III/IPI/ISI, iron + two paper) | **tested** — phase 14: output id and an emptied grid |
| allowed → activate without a code | **tested** — phase 14: `screen=playing`, `powered=true`, door `open=true`, no key press |
| not listed → the ordinary code screen | **tested** — phase 14: after swapping to `stranger`, `screen=container`, door closed |
| no code → allowlist cannot bypass the first set | **unit test** `an_allowed_player_activates_without_a_code` plus the `verifyPasscodeSet`-first order in `on_use`; not in-game (the harness sets a code first) |
| 50-player cap, case-insensitive duplicate-free list | **unit test** `the_allowlist_is_bounded_and_case_insensitive` |
| `affectEveryone` arm | **unit test** `an_allowed_player_activates_without_a_code`; no in-game editor (gap) |
| denylist module and its "denied → nothing" branch | **not ported** — the next slice (separate item/recipe/list; no denylist installed means `isDenied` is false, as here) |
| team lists, customize screen, module chat messages | **not ported** — no SDK team surface, no module screen; recorded as deviations |
| disguise / smart modules and the rest of the gap list | **not ported** — unchanged from slices 1/3/4 |

### Deviations recorded by this slice

1. **The item's id is `securitycraft:allowlist_module`, not the jar's `whitelist_module`.** The jar's
   source calls the module type `ALLOWLIST` while registering the item as `whitelist_module`; the
   brief requires the current name. The recipe shape, ingredients, texture and lang text are the
   jar's. Everything that already saved `whitelist_module` is unaffected (new id).
2. **Players are keyed by the 32-hex UUID used for `owner`, not by display name.** Upstream compares
   names case-insensitively; the pure rule keeps that comparison, and a single-player harness needs
   a deterministic stand-in for "the acting player" and "a stranger".
3. **`/securitycraft allow` sets the one list entry; upstream's customize screen adds/removes.**
   A harness addition, recorded as such, so one command can put the other side of the check in front
   of the keypad.
4. **No team list and no in-game `affectEveryone` editor.** The pure rule carries both arms, but
   the SDK exposes no team surface in this slice; only the player list is editable (harness command).
5. **Denylist is absent, so `isDenied` is always false.** That is upstream's behaviour for a keypad
   without that module, but the "denied → message and no activation" branch is a gap.
6. **No allowlist/denylist chat messages** (`sendAllowlistMessage` / `sendDenylistMessage` options
   are not configurable or carried); the port's panel has one status line.
7. **Module data is still ids + a separate player list, not an `ItemStack` component** (slice 3's
   deviation); installing and dropping are faithful, a component round-trip is not.

### Changelog for the resubmission (0.2.6)

> Supersedes #1805 (0.2.5). Adds the allowlist module carried from the jar (`whitelist_module`'s
> recipe, texture and lang, under the current `allowlist_module` id) and upstream's
> `isAllowed` branch: after `verifyPasscodeSet`, a listed player activates a coded keypad without
> typing a code, while everyone else still gets the code screen. A harness-only
> `/securitycraft allow` stands in for the customize screen, `/securitycraft info` reports
> `allow=` and both module ids, and the e2e proves the activation and its stranger differential with
> an inverted SELFTEST. The SHA-256 primitive, SDK-only dependency set and save format are unchanged.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review

## Next slice

**Family:** the **denylist module and its "denied → no interaction" branch** — the companion of this
slice, from the line just added above: *"denylist module and its 'denied → no interaction' branch |
not ported — the next slice (separate item/recipe/list; no denylist installed means `isDenied` is
false, as here)"*. It fits tier **deep** in the same logic+machines shape: a second carried list
module (`blacklist_module`'s recipe/texture/lang), one more id in `modules`, the same install/drop
machinery, a second harness list command and one new branch in `on_use` placed **before** the
allowlist branch, as upstream's `isDenied` check precedes `isAllowed`.

**Why it is the next one:** the list data, the 50-player cap, the case-insensitive comparison and the
install/drop paths already exist and are proved; only a second item id and the deny branch are new.
The disguise and smart modules need a customize screen / a code-cooldown change and remain behind it.

**In-game assertion that would prove it:** on a coded keypad with an adjacent door, install the
denylist module, set the acting player with `/securitycraft deny <pos> self`, then `ctl interact`:
`/securitycraft info` must read `powered=false` and `ctl block` on the door `open=false`, and the
click must leave `screen=playing` (upstream's denied click opens nothing). Counterfactual: swap the
list to `stranger` with the same command and the identical click must open the code screen
(`screen=container`), still with `powered=false` and the door closed. `SELFTEST=deny` must invert the
"denied click opened nothing" assertion and end `EXIT:1`. The existing allowlist phase must stay
green when neither module denies/allows an unlisted player.

## Slice 6 — the denylist module and "denied → no interaction" (2026-11-24, SDK `0a875da`) — 0.2.7

**Slice.** The companion gap from slice 5's coverage table, and this file's `## Next slice`:

> denylist module and its "denied → no interaction" branch | not ported — the next slice

This slice adds the second carried list module and the deny branch. Class **content**, tier
**deep** in the ledger; the delivered shape is the port's existing **logic + machines** half
(block entity `modules` list, install via `use_item_on`, drawn panel, redstone) plus one carried
module item with its recipe, model, texture and lang — no new block, renderer or hook kind.

### Upstream read (`Geforce132/SecurityCraft`, branch `1.21.1`, commit `c4d35b0c`)

The jar (`securitycraft-1.10.2.1.jar`) and the pinned commit's files, read for behaviour only:
`items/ModuleItem.java` + `api/IModuleInventory.java` (`isDenied` before `isAllowed`),
`components/ListModuleData.java` (`MAX_PLAYERS = 50`, case-insensitive player names),
`blocks/KeypadBlock.java` (`useWithoutItem` dispatch), the jar's
`assets/securitycraft/textures/item/blacklist_module.png` (506 B, extracted byte-for-byte) and
`data/securitycraft/recipe/blacklist_module.json` (`III / IPI / ISI`, `I = c:ingots/iron`,
`P = minecraft:paper`, **`S = minecraft:ink_sac`** — the allowlist uses paper), and the
`module.securitycraft.keypad.blacklist_module.description` lang line ("ban listed players from
interacting with the block").

### Red proof (TDD, before the rule existed)

`runtime::is_denied` was stubbed to `false`; `a_denied_player_is_refused` was added first. A
forwarded test step (`slice6-red.log`) failed:

```
---- runtime::tests::a_denied_player_is_refused stdout ----

thread 'runtime::tests::a_denied_player_is_refused' (147644) panicked at src/runtime.rs:1011:9:
a listed player is denied, upstream's ListModuleData.isPlayerOnList

test result: FAILED. 48 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

The rule became `installed && (affects_everyone || contains(player))` over the same 50-player,
case-insensitive, duplicate-free `ListData` slice 5 proved; **51 passed**, 0 failed.

### What the slice implements

- **The rule** (`runtime::is_denied`), pure and unit-tested. Deny is checked **before** allow,
  as upstream's `useWithoutItem` does, so a player on both lists is refused.
- **Content.** `securitycraft:denylist_module` (the jar's `blacklist_module` model, 16×16
  texture, III/IPI/ISI recipe with `minecraft:ink_sac` standing in for `c:ingots/iron`'s
  resolved iron, lang and creative-tab entry), using the 0.2.6 rename convention.
- **Install and drop.** The owner may install one denylist module on a keypad that does not
  have it (creative keeps the stack); the module drops both on a player break and on a
  level-side `/setblock` removal through the kept-list + `level_tick_post` path.
- **The branch** (`support::on_use`, after the `verifyPasscodeSet` equivalent and **before**
  the allowlist branch): a coded keypad whose denylist contains the acting player ends the
  click with `InteractionResult::Fail` (so the content block's menu never opens — `screen=playing`),
  sets `status=8` and leaves the keypad unpowered. A non-denied player falls through to the
  allowlist branch and then the ordinary code screen. An unset keypad never reaches either
  branch, so a list module cannot bypass the first code.
- **Harness control** `/securitycraft deny <pos> <self|stranger|32-hex>` (op 2), the
  counterpart of `allow`: sets the one list entry; only the owner may edit it. `/securitycraft
  info` gains `deny=<count>` and lists `denylist` in `modules=`.

### Verification (0.2.7, SDK `0a875da`, 2026-11-24)

Compile/test/format/check/build steps ran through **forwarded** `build-slot.sh` leases on
`AdLf-PC` (`build-slot: forwarding … to the remote runner`; hostname and paths quoted in
`slice5-remote-probe.log`) with the relative `--port ports/securitycraft`, `CARGO_INCREMENTAL=0`
and **no `BUILD_LOCAL=1`**. The launcher build and the game legs stayed local: the remote box
has no launcher, no 26.3 resources and no renderer for this `--offscreen` e2e. The launcher's
machine-wide native-build lock was held by other jobs during the selftest wave; the queued legs
waited for a slot and ran unchanged (no false failure was reported). Logs are under
`../_shared/evidence/securitycraft/` and `../_shared/evidence/securitycraft-e2e/`.

| Check | Command | Result |
|---|---|---|
| Red | forwarded `cargo test --offline` with `is_denied` stubbed | **48 passed / 1 failed** (`slice6-red.log`) |
| Tests | forwarded `cargo test --offline` | **51 passed**, 0 failed (`sdk-test.log`, regenerated for 0.2.7) |
| Formatting + lints | forwarded `cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`slice6-lint.log`) |
| Package ×2 | `rm -f Cargo.lock` before each; two builds + `cmp` | **byte-identical** (`slice6-build.log`) |
| Delivered | `securitycraft.hmod` | **67,128 B**, SHA-256 `7f1f7243129083bee719a20c14b2c232d2ed1807c4d9b539b2c28bb6aee48f7c`, **57 entries**, **no `Cargo.lock`** |
| Registry check | forwarded `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.7 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 (`slice6-check-fits.log`) |
| Pinned build | forwarded `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, EXIT 0, delivered SHA quoted (`slice6-check-fits.log`) |
| Another build | forwarded `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, EXIT 0, same SHA (`slice6-check-fits.log`) |
| Gate | `verify-sdk-package.py securitycraft` | **1/1**, incl. `package matches the tree`, SDK-only deps, no lock (`slice6-verify-sdk-package.log`) |
| Launcher build | `launcher-game.sh build securitycraft` | `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-91db09b0e4bc` (`slice6-launcher-build.log`) |
| In-game PASS | launcher-built game carrying 0.2.7 | **PASS + `EXIT:0`** (`securitycraft-e2e/e2e-launcher.log`), all 9 earlier legs + phase 14 + phase 15 green |
| Control | `GAME=$CONTROL_GAME`, unmodified Nightly | **`EXIT:1`** at phase 1 (`e2e-control.log`) |
| Ids-unknown control | `CONTROL=1` | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| SELFTEST (door/harm/nonloot/owner/relaunch/deny) | `SELFTEST=1\|harming\|nonloot\|owner\|relaunch\|deny` | **`EXIT:1`** at each own assertion (`e2e-selftest*.log`) |
| SELFTEST (allowlist) | `SELFTEST=allow` | **`EXIT:1`** at phase 14 ("expected an allowed click not to power the keypad") after one transparent retry (`e2e-selftest-allow.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=11 ok=11`, exit 0 (`slice6-check-e2e-logs.log`) |

Phase 16 quoted from `e2e-launcher.log`:

```
denylist recipe result: securitycraft:denylist_module
denylist grid slots still holding an item: 0
coded denylist keypad: securitycraft keypad 22 101 10 code=set entered= mode=check signal=0 powered=false status=2 modules=none allow=0 deny=0 owner=set door=21,101,10:closed
after installing the denylist module: … modules=denylist allow=0 deny=0 owner=set door=21,101,10:closed
after adding the acting player to the denylist: … modules=denylist allow=0 deny=1 owner=set door=21,101,10:closed
denied click: screen=playing … powered=false status=8 modules=denylist allow=0 deny=1 owner=set door=21,101,10:closed
after swapping the denylist to a stranger: … modules=denylist allow=0 deny=1 owner=set door=21,101,10:closed
non-denied click: screen=container … powered=false status=0 modules=denylist allow=0 deny=1 owner=set door=21,101,10:closed
```

Both value pairs are fail-capable: `SELFTEST=deny` inverts the "denied click opens nothing"
assertion on the delivered build; the stranger differential proves the same click would
otherwise open the code screen. `src/runtime.rs` (`928743606d28…`) is byte-identical inside
the package. The pre-existing phase-11b module-stash flake recorded under R9 did not recur in
the PASS leg. In this wave `SELFTEST=allow` **did** stop at phase 11b once
(`slice6-allow-flake-11b.log`: `items: []`, "the installed module vanished with a level-side
removal", `EXIT:1`) before reaching its own assertion; the identical leg was retried
transparently and then failed at its own inverted phase-14 assertion (`e2e-selftest-allow.log`,
`EXIT:1`). Both logs are kept; the flake is not called green.

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:denylist_module` item (craft, give, install, drop) | **tested** — phase 16 recipe (id/count + emptied grid), install, `all_three_installed_modules_drop` |
| the denylist recipe (III/IPI/ISI, iron + paper + ink sac) | **tested** — phase 16: output id and an emptied grid |
| denied → no interaction (no screen, no power, door closed) | **tested** — phase 16: `screen=playing`, `powered=false`, door `open=false`, `status=8` |
| non-denied → ordinary code screen | **tested** — phase 16: after swapping to `stranger`, `screen=container`, still unpowered/closed |
| deny before allow | **unit test** `a_denied_player_is_refused` + the branch order in `on_use`; the in-game keypad carries only a denylist, so precedence is structural |
| 50-player cap, case-insensitive duplicate-free denylist | **unit test** `the_denylist_is_bounded_and_case_insensitive` |
| `affectEveryone` arm | **unit test** `a_denied_player_is_refused`; no in-game editor (gap) |
| team lists, customize screen, module chat messages | **not ported** — no SDK team surface, no module screen; recorded as deviations |
| disguise / smart modules and the rest of the gap list | **not ported** — unchanged from slices 1/3/4/5 |

### Deviations recorded by this slice

1. **The item's id is `securitycraft:denylist_module`, not the jar's `blacklist_module`.** The
   same rename convention as 0.2.6's `whitelist_module` → `allowlist_module`; recipe shape
   (with the ink sac), texture and lang are the jar's.
2. **Players are keyed by the 32-hex UUID used for `owner`, not by display name.** The pure
   rule keeps upstream's case-insensitive name comparison; the harness needs a deterministic
   stand-in for "the acting player" and "a stranger".
3. **`/securitycraft deny` sets the one list entry; upstream's customize screen adds/removes.**
   A harness addition, recorded as such.
4. **No team list and no in-game `affectEveryone` editor.** The pure rule carries both arms;
   only the player list is editable (harness command).
5. **A denied click reads `status=8` in the readout**, not a `messages.securitycraft` chat
   line (the mod's message keys are not carried); the panel has one failure line.
6. **Module data is still ids + separate player lists, not an `ItemStack` component**
   (slice 3's deviation); installing and dropping are faithful, a component round-trip is not.
7. **The owner exception in `isDenied` is not modelled in the pure rule.** The acting player is
   the owner of the harness keypad and is the one being denied, exactly as upstream's
   `affectEveryone` arm denies everyone *except* the owner; the port's single-player harness
   cannot stand a non-owner in front of the keypad, so the exception is a gap recorded here
   rather than a behaviour claimed.

### Changelog for the resubmission (0.2.7)

> Supersedes #1805→0.2.6 (the staged PR). Adds the denylist module carried from the jar
> (`blacklist_module`'s recipe with an ink sac, texture and lang, under the current
> `denylist_module` id) and upstream's `isDenied` branch: after `verifyPasscodeSet`, a listed
> player's click opens nothing and powers nothing, while everyone else falls through to the
> allowlist branch and then the code screen. A harness-only `/securitycraft deny` stands in for
> the customize screen, `/securitycraft info` reports `deny=` and the new module id, and the e2e
> proves the refusal and its stranger differential with an inverted SELFTEST. The SHA-256
> primitive, SDK-only dependency set and save format are unchanged.

## Next slice

**Family:** the **smart module gate on the retry cooldown** — from slice 3's recorded deviation
("the port's throttle is unconditional since review #1197, so a keypad without the smart module
still rate-limits") and the gap line "the smart and harming modules; the disguise module". It fits
tier **deep** in the same logic+machines shape: one more carried module item (the jar's
`smart_module` recipe/model/texture/lang), one more id in `modules`, the same owner install/drop
paths, and a pure rule that makes the existing `Throttle` conditional on the module being
installed — no new block, renderer or hook kind.

**Why it is the next one:** the module install/drop machinery, the `modules` list and the
server-side `Throttle` already exist and are proved; only one item id and one gate are new. It
also closes a real upstream deviation rather than adding inert content. The disguise module
needs a client tint path, which this pin may not expose, so it stays behind it.

**In-game assertion that would prove it:** on a coded keypad without any module, enter five wrong
codes and then the right one — `/securitycraft info` must read `status=3`, `powered=true` and the
adjacent door `open=true` (upstream has no cooldown without the smart module). Install the smart
module on a second coded keypad, repeat the five wrong codes, and the right code must instead be
refused with `status=6`, `powered=false` and the door `closed`. `SELFTEST=smart` must invert the
first (no-module) assertion and end `EXIT:1`; the deny/allow phases must stay green in the same
leg. If the pin cannot express the per-player cooldown without the module, stop and report the
gap instead of faking it.

## Slice 7 — the smart module gate on the retry cooldown (2026-11-24, SDK `0a875da`) — 0.2.8

**Slice.** The `## Next slice` above, from slice 3's deviation *"the port's throttle is
unconditional since review #1197, so a keypad without the smart module still rate-limits"* and the
gap line *"the smart and harming modules; the disguise module"*. Class **content**, tier **deep**
in the ledger; the delivered shape is the port's existing **logic + machines** half plus one
carried module item, one more id in `modules`, the same owner install/drop paths and one pure rule
making the existing `Throttle` conditional on the module — **no new block, renderer or hook kind**.
The deny-before-allow order in `on_use` is untouched.

### Upstream read (the artefact wins over the brief's name)

The jar (`ports/_shared/carry-work/securitycraft/securitycraft-1.10.2.1.jar`) carries the item id
**`securitycraft:smart_module`** — the same name the brief used, but read from the artefact first:
`assets/securitycraft/models/item/smart_module.json` (93 B),
`assets/securitycraft/textures/item/smart_module.png` (545 B, sha256 `c20c93b4…bac8c`),
`data/securitycraft/recipe/smart_module.json` and the `item.securitycraft.smart_module` /
`module.securitycraft.keypad.smart_module.description` lang lines. The recipe is
`III / IPI / ISI` with `I = c:ingots/iron`, `P = minecraft:paper`, **`S = c:ender_pearls`** — a
different last ingredient from the harming (arrow), allowlist (paper) and denylist (ink sac)
modules; the port uses `minecraft:ender_pearl` as the pack's tag resolves.

The pinned commit's Java source (`Geforce132/SecurityCraft` `c4d35b0c`, read from
`src/main/java/net/geforcemods/securitycraft/`, behaviour only) gave the exact rule:

| File | What it says |
|---|---|
| `api/IPasscodeProtected.java` (`onIncorrectPasscodeEntered`, lines 298–306) | on a wrong code, `isModuleEnabled(ModuleType.SMART)` → `startCooldown()`; the harming module's damage is a separate `if` after it. |
| `api/Option.java` (`SmartModuleCooldownOption`, lines 290–293) | the cooldown is a tick count, default **100** (5 s), range 20–400; its lang description says "requires Smart Module". |
| `network/server/CheckPasscode.java` (`handle`) | two gates before hashing: the **per-player** 250 ms spam guard (`PasscodeUtils.isOnCooldown(player)`) and the **keypad** cooldown (`passcodeProtected.isOnCooldown()`); the latter is the one the smart module starts. |

The port's `Throttle` (review #1197) is its own form of the keypad cooldown: five wrong codes start
a ten-second lockout. This slice does not redesign it, it **gates it on the module**: without a
smart module upstream starts no keypad cooldown, so five wrong codes must still let the right code
open the keypad. The per-player 250 ms spam guard is not modelled (recorded as a deviation).

### Red proof (TDD, before the rule existed)

`cooldown_allows` / `cooldown_failed` were added with stubs that kept the old unconditional
behaviour, and two tests were written first. Forwarded `cargo test --offline` (`slice7-red.log`):

```
---- runtime::tests::a_keypad_without_the_smart_module_has_no_cooldown stdout ----

thread 'runtime::tests::a_keypad_without_the_smart_module_has_no_cooldown' (257375) panicked at src/runtime.rs:724:9:
five wrong codes must not rate-limit a keypad without the smart module
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- runtime::tests::the_smart_module_gates_the_existing_retry_budget stdout ----

thread 'runtime::tests::the_smart_module_gates_the_existing_retry_budget' (257408) panicked at src/runtime.rs:746:9:
the same budget is ignored without the module

test result: FAILED. 52 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

The rules became `!smart || throttle.allows(now_ms)` and `if smart { throttle.failed(now_ms) }`;
**54 passed**, 0 failed (`slice7-test.log`, regenerated into `sdk-test.log` for 0.2.8).

### What the slice implements

- **The pure rule** (`runtime::cooldown_allows`, `runtime::cooldown_failed`): the existing
  `Throttle` is consulted and filled only when a smart module is installed; no budget is recorded
  without one, so installing it later cannot inherit lockout failures earned before it was there.
  A correct code still calls `Throttle::succeeded()` and clears the budget.
- **The glue** (`support::apply_key`): the module is read from the saved `modules` list before the
  server's pre-comparison gate and before a wrong-code failure is recorded. A locked-out submit
  still returns before the digest comparison and reads `status=6`; the deny and allow branches in
  `on_use` and their order are unchanged.
- **Content.** `securitycraft:smart_module` (the jar's model, 16×16 texture and lang, the jar's
  recipe with `minecraft:ender_pearl` standing in for `c:ender_pearls`), its creative-tab entry,
  the resolved item key registered with the other three modules, and the drop paths
  (`block_changed` + `level_tick_post` and a player break).
- **Readout.** `/securitycraft info` now lists `smart` in `modules=`. No new harness command: the
  module is installed with the real held-item right-click, as the other three are.

### Verification (0.2.8, SDK `0a875da`, 2026-11-24)

Every Rust step ran in a **forwarded** `build-slot.sh` lease on `AdLf-PC` (`build-slot:
forwarding … to the remote runner`, relative `--port ports/securitycraft`, `CARGO_INCREMENTAL=0`,
**no `BUILD_LOCAL=1`**); the launcher build and the game legs stayed local (the e2e is
`--offscreen`). Logs are in `../_shared/evidence/securitycraft/` and
`../_shared/evidence/securitycraft-e2e/`.

| Check | Command | Result |
|---|---|---|
| Red | forwarded `cargo test --offline` with the rule stubbed | **52 passed / 2 failed** (`slice7-red.log`) |
| Tests | forwarded `cargo test --offline` | **54 passed**, 0 failed (`slice7-test.log`, `sdk-test.log`) |
| Format + lints | forwarded `cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`slice7-lint.log`) |
| Package ×2 | `rm -f Cargo.lock` before each; two builds + `cmp` | **byte-identical** (`slice7-build.log`) |
| Delivered | `securitycraft.hmod` | **69,773 B**, SHA-256 `b93884158f168af3e41b84dd700ce47befcdacecf49ea0e5a759bd744c504051`, **61 entries**, **no `Cargo.lock`** |
| Registry check | forwarded `cargo horizon-mod check securitycraft.hmod` | `securitycraft 0.2.8 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 (`slice7-check-fits.log`) |
| Pinned build | forwarded `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, EXIT 0, delivered SHA quoted (`slice7-check-fits.log`) |
| Another build | forwarded `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, EXIT 0, same SHA (`slice7-check-fits.log`) |
| Gate | `verify-sdk-package.py securitycraft` | **1/1**, incl. `package matches the tree`, SDK-only deps, no lock, 54 tests (`slice7-verify-sdk-package.log`) |
| Clean launcher build | stale 0.2.7 tag moved aside, then `launcher-game.sh build securitycraft` | `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-6abd9a7c8302` (`slice7-launcher-build.log`) |
| In-game PASS | launcher-built game carrying 0.2.8 | **PASS + `EXIT:0`** (`securitycraft-e2e/e2e-launcher.log`) |
| Control | `GAME=$CONTROL_GAME`, unmodified Nightly | **`EXIT:1`** at phase 1 (`e2e-control.log`) |
| Ids-unknown control | `CONTROL=1` | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| SELFTEST (smart) | `SELFTEST=smart` | **`EXIT:1`** at phase 3b, "expected a no-module keypad to be rate-limited" (`e2e-selftest-smart.log`) |
| SELFTEST (door/harm/nonloot/owner/relaunch/allow/deny) | `SELFTEST=1\|harming\|nonloot\|owner\|relaunch\|allow\|deny` | **`EXIT:1`** at each own assertion (`e2e-selftest*.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=13 ok=13`, exit 0 (`slice7-check-e2e-logs.log`) |
| Recipe types | `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

Phase 17 quoted from `e2e-launcher.log`:

```
smart recipe result: securitycraft:smart_module
smart grid slots still holding an item: 0
coded smart keypad: securitycraft keypad 10 101 -2 code=set entered= mode=check signal=0 powered=false status=2 modules=none allow=0 deny=0 owner=set door=11,101,-2:closed
after installing the smart module: … modules=smart allow=0 deny=0 owner=set door=11,101,-2:closed
right code with the smart module: securitycraft keypad 10 101 -2 code=set entered= mode=check signal=0 powered=false status=6 modules=smart allow=0 deny=0 owner=set door=11,101,-2:closed
```

and phase 3b in the same leg:

```
right code after five wrong codes, with no module: securitycraft keypad 13 101 0 code=set entered= mode=check signal=57 powered=true status=3 modules=none allow=0 deny=0 owner=set door=12,101,0:open
```

Both value pairs are fail-capable: `SELFTEST=smart` inverts the no-module `status=3` / powered /
door-open assertion and fails exactly there (`EXIT:1`); the module leg's own assertion is proved by
the differential between the two keypads in the same run. The allowlist, denylist, owner and
relaunch legs stayed green unchanged in this leg.

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:smart_module` item (give, craft, install, drop) | **tested** — phase 17 recipe (id/count + emptied grid), install, `all_four_installed_modules_drop` |
| the smart recipe (III/IPI/ISI, iron + paper + ender pearl) | **tested** — phase 17: output id and an emptied grid |
| with the module: five wrong codes then right → cooldown | **tested** — phase 17: `status=6`, `powered=false`, door `closed` |
| without the module: five wrong codes then right → opens | **tested** — phase 3b: `status=3`, `powered=true`, door `open`; inverted by `SELFTEST=smart` |
| a late install does not inherit old failures | **unit test** `a_keypad_without_the_smart_module_has_no_cooldown` |
| deny before allow is unchanged | **unit test** `a_denied_player_is_refused` + the unchanged branch order in `on_use`; phases 14/16 green in the PASS leg |
| the module's 100-tick default / configurable cooldown length | **not ported** — the port keeps review #1197's ten-second throttle, gated on the module; config is not exposed (recorded as a deviation) |
| customize screen (remove/toggle a module) | **not ported** — no module inventory screen |
| disguise module and the rest of the gap list | **not ported** — unchanged from slices 1/3/4/5/6 |

### Deviations recorded by this slice

1. **The cooldown is the port's review-#1197 throttle (five failures, ten seconds), not upstream's
   100-tick default.** Upstream starts a fresh fixed cooldown on each wrong code with the smart
   module; this port keeps the existing five-strike budget and merely makes it conditional on the
   module. The observable contract the slice proves is the same: no module → no keypad cooldown,
   module → the right code after five wrong ones is refused. The config option is not exposed.
2. **The per-player 250 ms spam guard is not modelled.** Upstream's `CheckPasscode` checks it
   before the keypad cooldown; the harness's key presses are several ticks apart, so it never
   decides an assertion here.
3. **The recipe's `S` is `minecraft:ender_pearl`**, the resolved member of the jar's
   `c:ender_pearls` tag (the same vanilla-id convention as the earlier modules' iron and arrow).
4. **Module data is still ids + separate player lists, not an `ItemStack` component** (slice 3's
   deviation); installing and dropping are faithful, a component round-trip is not.
5. **A transient first PASS-leg attempt stopped at phase 17's keypad placement** (`FAIL: the smart
   keypad did not place`, `EXIT:1`) after the smart recipe and install succeeded. No source or
   package byte changed before the retry; the retry only added a diagnostic echo of the target,
   inventory counts and the `ctl place` reply (extra ctl round-trips), after which the same place
   answered `changed:true` on air and the whole leg passed. Recorded, not called green; the
   terminal excerpt is `../_shared/evidence/securitycraft-e2e/e2e-smart-first-attempt-flake.log`.
   The previously recorded phase-11b module-stash flake did not recur in any leg this wave.

### Changelog for the resubmission (0.2.8)

> Supersedes #1805→0.2.7 (the staged PR). Adds the smart module carried from the jar
> (`smart_module`'s recipe with an ender pearl, texture, model and lang) and closes the deviation
> recorded in slice 3: upstream's keypad cooldown now applies only when that module is installed,
> so five wrong codes followed by the right one open a keypad without a module and are refused
> (`status=6`, unpowered, door closed) on one with it. The pure gate, the owner install/drop paths,
> the readout and two inverted-selftest phases prove it; the SHA-256 primitive, SDK-only dependency
> set, save format and deny-before-allow order are unchanged.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review, rust-port

## Slice 8 (0.2.9): the disguise module and its keypad tint

**Slice and gap line.** The last module-family gap from slice 3's line "the smart and harming
modules; the disguise module", now that harming, allowlist, denylist and smart are all ported. It
fits tier **deep**: it is a block-entity client render plus the carried item/recipe/texture/lang,
the same install/drop/readout paths as the other four modules, and no new block or machine.

**The pin gate — measured, not assumed.** Grep of `ports/_shared/sdk/0a875da/surface.json` is
**not zero-hit**: `register_block_tint_sources`, `register_item_tint_sources`,
`register_color_resolvers`, `block_entity_submit` (client, `entity: &ModBlockEntity` = the
server-synced saved data), `block_entity_update_tag` (server, sets what clients get) and the host's
`block_entity_sync(pos)`. `BlockEntityPart` carries both BE methods, so a mod can push the installed
modules to the client and draw from them. This is **not** buildinggadgets2's
`blocked(no-client-held-stack-components)` shape: that blocker was the *player's held item stack*,
which the client host cannot read with components (`player_inventory_slots` is server-only); here the
state driving the render lives in the **block entity**, which the client copy explicitly carries. A
block *tint source* cannot read it (`BlockAndTintGetter`: "Block entities are not part of the
region"), so the render is done in `block_entity_submit`, not as a tint-source callback.

**Red → green.** Adding the failing test first (`runtime::tests::a_disguised_keypad_tints_only_with_the_module`)
failed to compile: `error[E0425]: cannot find function 'disguise_tint' in this scope` (and the two
undefined constants), `slice8-red` via the forwarded slot. Defining `DISGUISE_MODULE`,
`DISGUISE_TINT` and `disguise_tint` in `runtime.rs` made it green. A second glue test,
`all_five_installed_modules_drop`, pins the fifth id through the drop path.

**What the jar supplied.** Item `securitycraft:disguise_module`, recipe III/IPI/ISI with
`I = minecraft:iron_ingot`, `P = minecraft:paper`, `S = minecraft:painting` (the jar's
`c:ingots/iron` and `minecraft:painting`, resolved to vanilla ids as for the other modules), the
item model, the 624-byte `disguise_module.png` texture and the en_us name/description. The
artefact's id wins over any name guessed from the mod's name.

**Render, as measured.** The first two in-game probes established, from frames and not from
reading Java: (1) `draw.cube`'s box is in **texels** (16 per block), not block units; (2) this engine
build does **not** multiply the cube's `tint` argument into its faces — a white-concrete cube tinted
`0xFF00FF00` rendered grey, and lime/red-concrete atlas sprites named through `CubeFace` drew
nothing. The working path is `host.block_key("minecraft:lime_concrete")` + `draw.block(...)`, which
draws the block's own green texels; block-entity models are unlit at this pin, so the green is dark
but green-dominant. The pure rule still gates the render on the installed module, and the server
pushes that state with `block_entity_sync(pos)` after the in-place install.

### Evidence (all forwarded to AdLf-PC except the launcher/e2e legs)

| Check | Evidence | Result |
|---|---|---|
| Red (forwarded `cargo test`) | new rule referenced before definition | `cannot find function 'disguise_tint'`, `EXIT:101` |
| Tests + clippy + fmt | `slice8-lint.log`, `sdk-test.log` (regenerated for 0.2.9) | **55 passed**, 0 failed, 0 warnings, fmt clean |
| Two builds + `cmp` | `slice8-build.log`, `slice8-build2.log` | byte-identical, `rm -f Cargo.lock` before each |
| Delivered package | `securitycraft.hmod` | **73,976 B**, SHA-256 `b9f614efd830c11a0e1b3416736ec841ea37855cb84cc7fa208b410b880`, **65 entries**, no `Cargo.lock`, version 0.2.9 |
| `cargo horizon-mod check` | `slice8-check-fits.log` | `securitycraft 0.2.9 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 |
| `fits --build 0a875da` | `slice8-check-fits.log` | `0a875da  made for`, EXIT 0 |
| `fits --build 0b14f8a` | `slice8-check-fits.log` | `0b14f8a  fits`, EXIT 0, same SHA |
| Gate | `slice8-verify-sdk-package.log` | **1/1**, incl. `package matches the tree`, SDK-only deps, no lock, 55 tests |
| Clean launcher build | stale `native-0a875da` tag moved aside, `launcher-game.sh build securitycraft` | `Compiling securitycraft (1 of 1)`, no registry crates, `native-0a875da-36c1a04c654d` (`slice8-launcher-build.log`) |
| In-game PASS | launcher-built game carrying 0.2.9 | **PASS + `EXIT:0`**, all 18 phases incl. phase 18 (`securitycraft-e2e/e2e-launcher.log`) |
| Control (unmodified Nightly) | `GAME=$CONTROL_GAME` | **`EXIT:1`** at phase 1, the package's ids are unknown (`e2e-control.log`) |
| Ids-unknown control | `CONTROL=1` | PASS + `EXIT:0`, both ids unknown (`e2e-ids-unknown.log`) |
| Prior SELFTESTs (door/harm/nonloot/owner/relaunch/allow/deny/smart) | `SELFTEST=1\|harming\|nonloot\|owner\|relaunch\|allow\|deny\|smart` | **`EXIT:1`** at each own assertion (`e2e-selftest*.log`) |
| SELFTEST (disguise) | `SELFTEST=disguise` | **`EXIT:1`** at phase 18, "expected the disguised keypad NOT to be green-dominant" (`e2e-selftest-disguise.log`) |
| Log checker | `check-e2e-logs.py securitycraft-e2e --strict` | `logs=15 ok=15`, exit 0 (`slice8-check-e2e-logs.log`) |
| Recipe types | `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

Phase 18 in the PASS leg (`e2e-launcher.log`):

```
disguise recipe result: securitycraft:disguise_module
disguise grid slots still holding an item: 0
plain keypad pixel: PIXEL 176 176 176 sampled=441 at=640,260
disguised keypad pixel: PIXEL 6 11 2 sampled=441 at=640,260
parsed RGB: plain=(176,176,176) disguised=(6,11,2)
green-dominant: plain=0 disguised=1 sum: plain=528 disguised=19
```

The two screenshots are `securitycraft-e2e/disguise-plain.png` and
`securitycraft-e2e/disguise-disguised.png`. The inverted leg read `plain=0 disguised=1` and stopped
exactly there at `EXIT:1`. One pre-existing e2e flake was diagnosed and removed: temporarily
widening `info()` to `ctl messages 30` consistently broke phase 2's second key sequence (the
code was never stored); reverting it to `messages 8` made the whole leg green again. The render
diagnostics (cube texel units, ignored tint argument, atlas sprite vs `draw.block`) are recorded
above and their frames kept as `diag-unconditional.png`.

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:disguise_module` item (give, craft, install, drop) | **tested** — phase 18 recipe (id/count + emptied grid), `modules=disguise` install, `all_five_installed_modules_drop` unit test |
| the disguise recipe (III/IPI/ISI, iron + paper + painting) | **tested** — phase 18: output id and an emptied grid |
| disguised keypad render (green-dominant pixel) | **tested** — phase 18: `--offscreen` screenshot of the same keypad before/after install, parsed RGB in the log; inverted by `SELFTEST=disguise` |
| deny-before-allow order, smart-module gate | unchanged — unit tests + phases 3b/14/16/17 green in the PASS leg |
| disguise block chosen by a module GUI | **not ported** — the pin stamps no module screen; the port fixes one appearance (deviation below) |

### Deviations recorded by this slice

1. **The disguise appearance is fixed, not GUI-chosen.** Upstream's `DisguiseModuleScreen` and
   `SetStateOnDisguiseModule` let a player pick any block state; no module screen is stamped at this
   pin, so the port renders a fixed lime-concrete disguise. The delivered behaviour is the visible
   disguise (a green-tinted keypad), not the selection UI.
2. **The cube's `tint` argument is ignored by this engine build** (measured above), so green comes
   from the block's own sprite; `DISGUISE_TINT` remains the pure rule's declared colour.
3. **The block-entity model is unlit**, so the green is dark but unambiguously green-dominant; the
   in-game assertion is a ratio/brightness comparison, not an absolute colour match.

### Changelog for the resubmission (0.2.9)

> Supersedes the staged 0.2.8 record. Carries the jar's `disguise_module` (recipe with a painting,
> texture, model and lang) as a fifth module through the same owner install/drop paths, and renders
> a disguised keypad with a green block-entity model: an `--offscreen` screenshot of the same keypad
> before and after installing the module differs in green-dominance and brightness, inverted by
> `SELFTEST=disguise`. The salted SHA-256, SDK-only dependency set, save format, deny-before-allow
> order and smart-module gate are unchanged.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review, rust-port

## Next slice

**Family:** the remaining **content families in the gap list** (the passcode chest/barrel/furnace/
smoker, keypad door and trapdoor, the universal block modifier), or, if the disguise selection UI is
wanted, a re-check at a pin that stamps a module screen. The disguise module family is now closed:
the five module ids are carried, proven in game and packaged. Before any new gap, re-read the pin
delta (`check-pin-updates.py`) — a module-screen or item-component surface would lift the two
recorded deviations. The in-game assertion for a carried container would be a placed block opened by
its owner with its code, reading its saved contents and an unknown id refused, with a differential
on an unprotected vanilla container.

## Slice 9 — the passcode barrel (2026-11-24, SDK `0a875da`) — 0.2.10

**Slice and gap line.** The last section's own `## Next slice` and the gap list's
*"the passcode chest/barrel/furnace/smoker, keypad door and trapdoor"*. From
that list this slice takes **the passcode barrel** (`securitycraft:keypad_barrel`),
the cheapest container the pin can express. Class **content**, tier **deep** in
the ledger; the delivered shape is the port's existing **logic + machines** half
(a block with saved state and a code-check path, no new entity or renderer beyond
what the block needs).

**The jar inventory — chosen from the artefact, not a name.** The jar
(`ports/_shared/carry-work/securitycraft/securitycraft-1.10.2.1.jar`, SHA-1
`8760079e…3251`) carries these passcode container families:
`keypad_barrel` (blockstate, two block models, five textures, loot table),
`keypad_chest` / `keypad_chest_double` (entity-rendered chest textures),
`keypad_furnace`, `keypad_smoker`, `keypad_blast_furnace` (lit + open states,
cook-time machines), `keypad_trapdoor`, `keypad_door`, `keypad_frame`, and the
`furnace_mine` / `smoker_mine` / `blast_furnace_mine` mine variants. The
**barrel** is the cheapest faithful choice: a 12-state blockstate (6 `facing`
× 2 `open`) against 26.3's barrel template, two models, five textures and one
loot table; no lit states, no double block, no entity renderer. The jar's own
48-variant blockstate (frog / lid_facing) is trimmed to the two properties the
target version has. Its item model is the jar's `keypad_barrel`.

**The pin gate — measured, not assumed.** The SDK stamps a 27-slot content menu
(`data/<id>/horizon/menu/`) and `Registrar::block_container`; the game backs its
slots with the mod block entity and the 27-slot screen is drawn through the
`screen` Part. This is the same path the keypad's 1-slot menu uses, so no new
menu surface is invented. The first probes established that a *native* menu
opened with `PlayerRef::open_menu` shows empty slots whose clicks do not reach
the block entity, and that `useItemOn` asks `useWithoutItem` next — so the
one-shot open gate must not be consumed by the first use call. Both findings
changed the implementation; the final leg proves the block-backed menu.

### Red proof (TDD, before the rule existed)

`src/runtime.rs` gained `ContainerAccess`, `container_access` (stubbed to
`WrongCode`) and `a_passcode_container_opens_for_its_owner_with_the_code`
first. Forwarded `cargo test --offline` (`slice9-red.log`) failed:

```
---- runtime::tests::a_passcode_container_opens_for_its_owner_with_the_code stdout ----

thread 'runtime::tests::a_passcode_container_opens_for_its_owner_with_the_code' (542393) panicked at src/runtime.rs:1190:9:
assertion `left == right` failed: the owner with the right code opens the container
  left: WrongCode
 right: Open

test result: FAILED. 55 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

The rule became "owner first, then salted digests via the existing `submit`,
then an open/wrong verdict"; `container_access` is pure and uses the same
`may_set_code`/`submit` paths as the keypad, so deny-before-allow, the smart
gate, the SHA-256 digest and the disguise render are untouched. A glue test
(`the_barrel_container_uses_a_27_slot_mod_menu`) pins the slot count and menu.
All **57** tests pass.

### What the slice implements

- **The pure rule** (`runtime::container_access`, `ContainerAccess`): a stranger
  is refused before any digest work, a coded container with no code cannot be
  checked, and only the owner's matching salted SHA-256 grants an open.
- **Content.** `securitycraft:keypad_barrel` (the jar's block/item ids, 12-state
  blockstate, two models, five 16×16 textures, loot table, lang and creative-tab
  entry), plus the 27-slot menu type `securitycraft:keypad_barrel` and the
  pickaxe-tag entry. The jar's five textures were extracted byte-for-byte.
- **The gate.** The barrel's `BlockPart` cancels a right-click while no code has
  granted an open; once granted, it returns `None` and the carried barrel
  template opens its 27-slot menu. The grant is a saved integer, cleared when
  that menu closes (so one code entry opens one session). The two use hooks and
  the close hook are separate Part registrations — one Part type per block.
- **Harness controls** (recorded as harness additions, not gameplay):
  `/securitycraft cset <pos> <digits>` (owner sets the first code),
  `/securitycraft copen <pos> <digits>` (checks it and grants the one-shot open)
  and `/securitycraft cinfo <pos>` (reads `code=`, `open=`, `owner=`). A headless
  harness cannot type into a second drawn screen; the port already drives code
  entry through commands for the keypad.

### Verification (0.2.10, SDK `0a875da`, 2026-11-24)

Every Rust step ran in a **forwarded** `build-slot.sh` lease on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`, relative
`--port ports/securitycraft`, `CARGO_INCREMENTAL=0`, **no `BUILD_LOCAL=1`**);
the launcher build and the game legs stayed local (the e2e is `--offscreen`).
Logs are in `../_shared/evidence/securitycraft/` and
`../_shared/evidence/securitycraft-e2e/`.

| Check | Evidence | Result |
|---|---|---|
| Red | `slice9-red.log` | **55 passed / 1 failed** (the new rule) |
| Tests | `slice9-test.log`, `sdk-test.log` (regenerated for 0.2.10) | **57 passed**, 0 failed |
| Format + lints | `slice9-fmt.log`, `slice9-clippy.log` | exit 0, **0 warnings** |
| Two builds + `cmp` | `slice9-build.log` | **byte-identical**; `rm -f Cargo.lock` before each |
| Delivered | `securitycraft.hmod` | **83,579 B**, SHA-256 `2829d08f2d21a71db2e47381215d0d50f3fbdaba0c18ee29b632fa20e1d41841`, **78 entries**, no `Cargo.lock`, version 0.2.10 |
| Registry check | `slice9-check-fits.log` | `securitycraft 0.2.10 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 |
| Pinned build | `slice9-check-fits.log` | `0a875da  made for`, EXIT 0 |
| Another build | `slice9-check-fits.log` | `0b14f8a  fits`, EXIT 0, same SHA |
| Gate | `slice9-verify-sdk-package.log` | **1/1**, incl. `package matches the tree`, SDK-only deps, no lock, 57 tests |
| Launcher build | `slice9-launcher-build.log` | `Compiling securitycraft (1 of 1)`, `native-0a875da-fa8e073dbac3 built` |
| In-game PASS | `securitycraft-e2e/e2e-launcher.log` | **PASS + `EXIT:0`**, all 19 phases incl. phase 19 |
| Control (unmodified Nightly) | `e2e-control.log` | **`EXIT:1`** at phase 1, the package's ids are unknown |
| Ids-unknown control | `e2e-ids-unknown.log` | PASS + `EXIT:0`, both ids unknown |
| Prior SELFTESTs (`1`/harming/nonloot/owner/relaunch/allow/deny/smart/disguise) | `e2e-selftest*.log` | **`EXIT:1`** at each own assertion |
| SELFTEST (container) | `e2e-selftest-container.log` | **`EXIT:1`** at phase 19, "expected a wrong code to leave the barrel's contents unreadable" |
| Log checker | `slice9-check-e2e-logs.log` | `logs=16 ok=16`, exit 0 |
| Recipe types | `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

Phase 19 in the PASS leg (`e2e-launcher.log`):

```
securitycraft barrel 13 101 12 code=set open=0 owner=set
wrong-code click: screen=playing container slots=0
securitycraft barrel 13 101 12 code=set open=1 owner=set
unlocked interact: … reply={...,"ok":true,"screen":"container"}
unlocked barrel container slots: 27
saved barrel slot 300: minecraft:iron_ingot x8
after closing: securitycraft barrel 13 101 12 code=set open=0 owner=set
unknown id reply: {"error":"unknown item securitycraft:no_such_block"}
barrel slot 300 after the relaunch: minecraft:iron_ingot x8
```

Both value pairs are fail-capable: `SELFTEST=container` inverts the "wrong code
opens nothing" assertion and fails exactly there; the vanilla barrel opened at
the same position without any code (27 slots) is the differential, and a wrong
code's click left `screen=playing` with zero container slots. The relaunch leg
reads the saved slot from the block entity after a fresh game start. Several
pre-existing harness flakes (a placement at phase 14/17 and one health read at
phase 10) stopped intermediate legs; each identical leg was retried unchanged
and the flake logs remain in `securitycraft-e2e/`. `ctl messages` stays at **8**
(the widened `30` broke phase 2 in slice 8).

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:keypad_barrel` block/item (give, place, read back) | **tested** — phase 19: `block_name` and `block_state` |
| owner recorded on placement | **tested** — phase 19: `owner=set` |
| the owner sets the first code; a second set is refused | **tested** — phase 19: `code=set`, "a second set" |
| wrong code → no permission, no menu, no slots | **tested** — phase 19: `open=0`, `screen=playing`, 0 slots; inverted by `SELFTEST=container` |
| right code → 27-slot container menu opens | **tested** — phase 19: `screen=container`, 27 slots |
| saved contents survive a restart | **tested** — phase 19: `minecraft:iron_ingot x8` before and after relaunch |
| an unknown id is refused | **tested** — phase 19: parsed `{"error":"unknown item …"}` |
| unprotected vanilla barrel (differential) | **tested** — phase 19: 27 slots, no code |
| the pure owner/code rule | **unit test** `a_passcode_container_opens_for_its_owner_with_the_code` |
| keypad chest / furnace / smoker / blast furnace, keypad door / trapdoor | **not ported** — the remaining gap list families |
| a drawn code GUI for the barrel | **not ported** — harness command recorded as a deviation below |
| disguise/allowlist/denylist/smart/harming modules on a barrel | **not ported** — modules install on keypads only |

### Deviations recorded by this slice

1. **The barrel's code check is a harness command, not a drawn second screen.**
   Upstream reuses the keypad's check GUI for every passcode container; the
   pin *can* stamp a screen, but a single `horizon-ctl` harness cannot type into
   a second panel while driving the keypad phases. The port's established
   convention for such paths is a harness command (`/securitycraft code`,
   `/owner`, `/allow`, `/deny`); `/securitycraft cset|copen|cinfo` follows it.
2. **A content menu type, not the vanilla `minecraft:generic_9x3` id.** A native
   menu opened with `PlayerRef::open_menu` showed empty slots whose clicks did
   not reach the block entity; the barrel therefore declares its own
   `securitycraft:keypad_barrel` 27-slot menu and draws a vanilla-shaped panel.
   The slot layout, count and saved items are those of a 9×3 chest.
3. **The one-shot grant is cleared at menu close, not at the granting right
   click.** `useItemOn` asks `useWithoutItem` next; consuming the grant in the
   first call made the second call cancel the very click that should open the
   menu (measured). The saved `open` integer is cleared by
   `player_container_close`, so one code entry opens one session.
4. **The blockstate is trimmed to 26.3's two properties.** The jar's
   `keypad_barrel` has 48 variants (`facing`, `open`, `frog`, `lid_facing`);
   the target version has only `facing` and `open`, so the carried file has 12.
5. **The loot table is simplified**: a survives-explosion pool dropping the
   barrel item, without the jar's `copy_components` custom-name transfer.
6. **Only the barrel is ported from the container family.** The chest needs an
   entity-rendered lid, and the furnace/smoker need lit states and cook-time
   machines; neither fits "the cheapest family the pin can express" as directly.

### Changelog for the resubmission (0.2.10)

> Supersedes the staged 0.2.9 record. Carries the jar's `keypad_barrel` (12-state
> blockstate, two models, five textures, loot table and lang) as a 27-slot
> passcode container: its placer is the owner, only the owner may set the first
> code, a wrong code leaves it closed with no readable slots, and the right code
> opens a 9×3 panel whose saved contents survive a relaunch. A new pure
> owner-and-digest access rule and a one-shot open gate are added; the SHA-256
> primitive, SDK-only dependency set, save format, deny-before-allow order,
> smart-module gate and disguise render are unchanged.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-review,rust-port

## Next slice

**Family:** the **passcode chest** (`securitycraft:keypad_chest`), the closest
companion of this barrel and the next line of the gap list
(*"the passcode chest/barrel/furnace/smoker"*). It fits tier **deep** in the same
logic+machines shape: a carried block with saved owner/code state, the same pure
`container_access` rule, the same one-shot gate and harness commands, a new block
id with the jar's entity-chest textures and a 27-slot menu — no new entity or
renderer beyond what the chest block requires. The barrel's machinery is now
proved, so only the chest's id/content and its double-chest loot need porting.

**In-game assertion that would prove it:** place a passcode chest, set its code
as the owner with `/securitycraft cset`, confirm a wrong `/securitycraft copen`
leaves `open=0`, `screen=playing` and zero container slots; the right code then
opens 27 slots, eight deposited ingots read back as `minecraft:iron_ingot x8`,
and after a quit-and-relaunch the same code opens the chest with the slot still
holding eight ingots. The differential is a vanilla chest at the same position
(the container opens with no code). `SELFTEST=chest` inverts the wrong-code
assertion and must end `EXIT:1`. If the chest's entity-rendered lid cannot be
expressed at this pin, stop with `blocked(<symbol>)` and evidence rather than
inventing a renderer.

## Slice 10 — the passcode chest is `blocked(register_block_entity_renderers)` (2026-11-24)

**Outcome: STOP at the artefact-inventory gate.** No version bump, no build, no
package, no e2e. The delivered package stays **0.2.10**
(`2829d08f…41841`) and its evidence stays as recorded in slice 9. The brief's
own stop condition applied: the chest needs an entity renderer the pin cannot
express, and substituting a `minecraft:chest`-template fake was explicitly
forbidden. Surface evidence:
[`slice10-chest-blocked-surface.log`](../../_shared/evidence/securitycraft/slice10-chest-blocked-surface.log).

### Artefact inventory (the jar, not the mod's name)

From `securitycraft-1.10.2.1.jar` (SHA-1 `8760079e…3251`), the chest family is:

- `assets/securitycraft/blockstates/keypad_chest.json` — 4 `facing` variants,
  one model `securitycraft:block/keypad_chest`.
- `assets/securitycraft/models/block/keypad_chest.json` — **`"parent":
  "builtin/entity"`**, only a `particle` texture (`block/iron_block`). No cube
  geometry, no textured elements — unlike the barrel's carried cube model.
- `assets/securitycraft/models/item/keypad_chest.json` — **`"parent":
  "builtin/entity"`**, display transforms only.
- Ten `assets/securitycraft/textures/entity/chest/*.png`: `active`,
  `inactive`, `left_active`, `left_inactive`, `right_active`,
  `right_inactive`, `christmas{,_left,_right}` (all 16×16).
- `net/geforcemods/securitycraft/renderers/KeypadChestRenderer.class` and
  `KeypadChestItemRenderer.class`, plus `KeypadChestBlockEntity.class` and the
  `KeypadChestBlock$Convertible` double-chest class.

The visible chest — the lid, its open/closed angle, the active/inactive and
left/right texture swap — is **entirely drawn by the two Java entity renderers**.
There is no static JSON mesh to carry; `builtin/entity` delegates the block's
geometry to a `BlockEntityRenderer` and the item's to an item renderer.

### The missing pin surface (measured, not assumed)

- The SDK's only render-registration hooks are `register_debug_renderers`,
  `register_entity_renderers` (entity **types**), `register_render_pipelines`
  and `register_special_model_renderer` (`minecraft:special` **item** models).
  There is **no `register_block_entity_renderers`** hook; the only block-entity
  render surface is `block_entity_render_bounding_box` (a bounds answer, not
  geometry). `surface.json` at `0a875da` was enumerated in full.
- The content `Block` declaration (`horizon-mods/src/content.rs`) has no renderer
  or texture-override field; block visuals come from the vanilla template plus
  the carried blockstate/model assets.
- The game has **no `builtin/entity` handling** anywhere in `horizon/crates/`,
  so carrying the jar's block/item model would draw no geometry from the chunk
  mesh.
- The game's block-entity chest renderer classifies by the block state's **own
  id** (`classify(&s.name, …)`) and matches only vanilla names (`chest`,
  `trapped_chest`, `ender_chest`, the copper chests); a `securitycraft:keypad_chest`
  block is not a chest to it. Those vanilla names use **hardcoded** textures
  `entity/chest/normal{,_left,_right}`; there is no state-driven
  active/inactive swap for a content block and no SDK binding for the mod's
  `entity/chest/*.png` textures.

Generic world-geometry hooks (`add_section_geometry`, `submit_custom_geometry`)
could only draw a hand-built mesh — i.e. inventing a renderer, which the brief
forbids — and still would not give the item's `builtin/entity` model a draw path.

### Why this is blocked rather than a thinner slice

The pure `container_access` rule, the one-shot gate, the 27-slot menu and the
harness commands are all reusable, but they port the **mechanics**, not the
chest. The artefact's defining content is the entity-rendered lid; a
`minecraft:chest`-template block carrying the mod's textures unused would be the
"fake barrel" the brief rules out, and the parity rule would list its entire
visual family as dead data. Stopping here keeps the recorded 0.2.10 package and
its 16 green e2e logs intact.

### Next slice proposal

The same gap line (*"…chest/barrel/**furnace/smoker**…"*): the **passcode
furnace/smoker** (`securitycraft:keypad_furnace`, `keypad_smoker`). Unlike the
chest, the pin has first-class `minecraft:furnace`/`minecraft:smoker`/
`minecraft:blast_furnace` container templates with cook-time machine behaviour
(`horizon-client-render::containers::FurnaceKind`, `recipe_type()`), so their lit
states and cook timers are expressible without a new renderer. Before dispatch
the next agent must inventory the furnace's blockstate/models/textures in the
jar and confirm the template's lit-state property names match 26.3, stopping
with `blocked(<symbol>)` if the artefact again needs a renderer.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices (no Rust change — artefact/surface audit only)

## Slice 11 — the passcode furnace (2026-11-24, SDK `0a875da`) — 0.2.11

**Slice and gap line.** The brief's own line, from slice 10's proposal and the port's gap list:
*"the passcode chest/barrel/**furnace/smoker**, keypad door and trapdoor"*. The chest stays
`blocked(register_block_entity_renderers)` (slice 10, measured — `builtin/entity` plus Java
renderers, no pin surface); this slice takes the **passcode furnace**
(`securitycraft:keypad_furnace`). Class **content**, tier **deep** in the ledger; the delivered
shape is the port's existing **logic + machines** half (block entity, container menu, redstone-free
one-shot gate) plus one carried block with cook-time state — no renderer, no new hook kind, no
re-tiering.

### Artefact inventory first (the gate the brief requires)

From the jar (`securitycraft-1.10.2.1.jar`, SHA-1 `8760079e…3251`), the `keypad_furnace` family:

| File | What the artefact contains |
|---|---|
| `assets/securitycraft/blockstates/keypad_furnace.json` | 16 variants: `facing`(4) × `lit`(2) × `open`(2), models `keypad_furnace_{closed,closed_lit,open,open_lit}` with quarter-turn `y` |
| `assets/securitycraft/models/block/base_keypad_furnace_closed.json` / `_open.json` | `"parent": "block/block"`, **10 `elements` with real cube geometry**, `render_type: cutout`, `#window`/`#furnace`/`#keypad` textures |
| the four `keypad_furnace_*` block models | parents to the two bases; `#furnace` = `block/furnace_front` / `block/furnace_front_on` (vanilla 26.3 textures), `#window` = the mod's window |
| `assets/securitycraft/models/item/keypad_furnace.json` | parents to `keypad_furnace_open` (a block model), display transforms only |
| five `assets/securitycraft/textures/block/keypad_furnace_*.png` | bottom, side, top_active, top_inactive, window (16×16) |
| `data/securitycraft/loot_table/blocks/keypad_furnace.json` | survives-explosion pool dropping `securitycraft:keypad_furnace` with `copy_components(custom_name)` |

**No model is `builtin/entity`** — checked with a per-file grep over all twelve furnace and smoker
block/item models (`slice10-furnace-inventory.log`): every one is `CLEAN`. The visible machine is a
static JSON mesh, unlike the chest (slice 10's blocker). **The lit-state property matches 26.3**:
the prepared `assets/minecraft/blockstates/furnace.json` has exactly `facing` × `lit`, and its
`textures/block/furnace_front{,_on}.png` are present. The jar's extra `open` property has no 26.3
counterpart, so the carried blockstate keeps the 8 `facing`/`lit` variants and the closed mesh for
both (the lid-open art is not expressible — recorded as a deviation). This is the same trim the
barrel's `frog`/`lid_facing` needed.

**Choice — furnace, not smoker.** The jar's smoker reuses `base_keypad_furnace_{closed,open}` and
adds only its own `keypad_smoker_window` texture, its own 16-variant blockstate and its own id/loot;
the furnace is the base artefact, has the full five-texture set, and is the first named in the gap
line. Both are expressible (the pin stamps `FurnaceKind::{Furnace,Smoker}` container behaviour —
`horizon-client-render::containers::FurnaceKind::recipe_type()`), so the cheaper, canonical furnace
is ported and the smoker stays a gap.

**Cook behaviour — expressible without a renderer.** `FurnaceKind::from_block` only classifies
vanilla ids, so a mod block gets a `BlockEntity::Mod`, not the engine's furnace BE; but the SDK
exposes everything the port needs: `Registrar::block_container`, `LevelRef::recipe_for` (the
engine's `minecraft:smelting` matcher), `Host::item_burn_time`, `Host::item_crafting_remainder` and
`set_block_property("lit", …)`. `ironfurnaces` already proves this exact pattern at this pin, so
the port drives cooking from its own block-entity tick with the game's own recipes and fuel values.

### Red proof (TDD, before the rule or glue existed)

`src/runtime.rs` gained `FurnaceState`/`FurnaceTick`/`FurnaceStep` and two tests, with
`furnace_access` stubbed to `false` and `furnace_tick` stubbed to `FurnaceStep::default()`. The
forwarded test step (`slice10-red.log`) failed:

```
---- runtime::tests::a_furnace_opens_for_its_owner_with_the_right_code stdout ----

thread 'runtime::tests::a_furnace_opens_for_its_owner_with_the_right_code' (948881) panicked at src/runtime.rs:1309:9:
the owner with the right code opens the furnace

---- runtime::tests::a_furnace_smelt_finishes_after_the_matched_recipes_time stdout ----

thread 'runtime::tests::a_furnace_smelt_finishes_after_the_matched_recipes_time' (948882) panicked at src/runtime.rs:1344:9:
the input smelts once the matched recipe's cooking time has passed

test result: FAILED. 57 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

The access rule became the existing owner-then-salted-digest verdict for the new id; the cook rule
became the burn/cook timer that smelts one input when the matched recipe's time completes. All
**59** tests then passed.

### What the slice implements

- **Pure rules** (`runtime::furnace_access`, `furnace_tick`, `FurnaceState`/`FurnaceTick`): owner
  before digest, and `AbstractFurnaceBlockEntity.serverTick`'s burn/cook/consume/smelt branch with
  no SDK types in `runtime.rs`.
- **Content**: `securitycraft:keypad_furnace` (the jar's block/item ids, 8-state blockstate, six
  models, five 16×16 textures, simplified loot, lang and creative-tab entry, pickaxe tag), plus a
  three-slot `securitycraft:keypad_furnace` menu and a drawn input/fuel/output panel.
- **The gate**: the same one-shot saved permission as the barrel — a wrong `copen` ends the click
  before the content block opens its menu, so the machine shows no slots; the right code grants one
  session, spent at menu close. The harness commands `cset`/`copen`/`cinfo` now dispatch by id and
  report `securitycraft barrel …` / `securitycraft furnace …`.
- **The cook tick**: matches `minecraft:smelting` for the input slot through `LevelRef::recipe_for`,
  reads the fuel slot's burn time through `Host::item_burn_time`, advances the four saved timing
  ints, consumes fuel, smelts one input into the output slot, and sets `lit` from `is_burning`. The
  saved ints and item slots survive a relaunch.

### Deviations recorded by this slice

1. **The `open` blockstate property is trimmed.** 26.3's furnace has only `facing` and `lit`; the
   jar's lid-open art (`base_keypad_furnace_open` and its models) is carried but unreferenced by the
   blockstate. The visible lid angle is not expressible; the menu opening is the observable "open".
2. **The loot table is simplified** to a survives-explosion drop of the furnace item, without the
   jar's `copy_components` custom-name transfer (as the barrel's was).
3. **The code check is a harness command** (`cset`/`copen`/`cinfo`), not a drawn second screen, as
   for every other passcode family in this port.
4. **Modules install on keypads only**, not on the furnace; upstream's per-container module lists
   remain a gap.

### Verification (0.2.11, SDK `0a875da`, 2026-11-24)

Every Rust step ran through forwarded `build-slot.sh` leases on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`, relative `--port ports/securitycraft`,
`CARGO_INCREMENTAL=0`, **no `BUILD_LOCAL=1`**); the launcher build and the game legs stayed local
(the e2e is `--offscreen`). Evidence under `../_shared/evidence/securitycraft/` and
`../_shared/evidence/securitycraft-e2e/`.

| Check | Evidence | Result |
|---|---|---|
| Red | `slice10-red.log` | **57 passed / 2 failed**, both new furnace assertions |
| Tests | `slice10-test.log`, `sdk-test.log` (regenerated for 0.2.11) | **59 passed**, 0 failed |
| Format + lints | `slice10-lint.log` | exit 0, **0 warnings**, fmt clean |
| Two builds + `cmp` | `slice10-build.log` | **byte-identical**; `rm -f Cargo.lock` before each |
| Delivered | `securitycraft.hmod` | **95,921 B**, SHA-256 `b3271e8884e04fc0a878eaf90a171bb7d49f8a6c34f36c94993448879a7e6f67`, **95 entries**, no `Cargo.lock`, version 0.2.11 |
| Registry check | `slice10-check-fits.log` | `securitycraft 0.2.11 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 |
| Pinned build | `slice10-check-fits.log` | `0a875da  made for`, EXIT 0 |
| Another build | `slice10-check-fits.log` | `0b14f8a  fits`, EXIT 0, same SHA |
| Gate | `slice10-verify-sdk-package.log` | **1/1**, incl. `package matches the tree`, SDK-only deps, no lock, 59 tests |
| Launcher build | `slice10-launcher-build.log` | stale 0.2.10 tag moved aside, `Compiling securitycraft (1 of 1)`, `native-0a875da-9c1c70931f1c built` |
| In-game PASS | `securitycraft-e2e/e2e-launcher.log` | **PASS + `EXIT:0`**, all 20 phases incl. phase 20 |
| Control (unmodified Nightly) | `e2e-control.log` | **`EXIT:1`** at phase 1 |
| Ids-unknown control | `e2e-ids-unknown.log` | PASS + `EXIT:0`, ids unknown |
| SELFTEST (furnace) | `e2e-selftest-furnace.log` | **`EXIT:1`** at phase 20, "expected a wrong code to leave the furnace's contents unreadable" |
| Prior SELFTESTs (door/harm/nonloot/owner/relaunch/allow/deny/smart/disguise/container) | `e2e-selftest*.log` | **`EXIT:1`** at each own assertion |
| Log checker | `slice10-check-e2e-logs.log` | `logs=17 ok=17`, exit 0 |
| Recipe types | `check-recipe-types.py securitycraft` | `securitycraft: ok`, exit 0 |

Phase 20 in the PASS leg (`e2e-launcher.log`):

```
unprotected vanilla furnace container slots: 3
coded furnace: securitycraft furnace 24 101 12 code=set open=0 owner=set lit=false
wrong-code furnace click: screen=playing container slots=0
after the right furnace code: securitycraft furnace 24 101 12 code=set open=1 owner=set lit=false
unlocked furnace container slots: 3
furnace output slot 302: minecraft:iron_ingot x1
furnace state while burning: securitycraft:keypad_furnace[facing=west,lit=true]
furnace input slot 300 after the cook: empty
furnace output slot 302 after the relaunch: minecraft:iron_ingot
PASS-furnace: the passcode furnace saved its owner and code, a wrong code left no menu and no slots, the right code opened its three-slot block-backed menu, raw iron and coal cooked into one iron ingot with the input consumed and lit=true, and the code and output survived a relaunch; an unprotected vanilla furnace opened with no code
```

The wrong-code pair (slots 0, `screen=playing`) is fail-capable: `SELFTEST=furnace` inverts it and
fails at `EXIT:1`. The vanilla furnace at the same position opens with no code (differential). The
cook is driven by the game's own `minecraft:smelting` matcher and coal's burn time; the input is
consumed and the single iron ingot is read back from slot 302 before and after the relaunch.
`ctl messages` stays at **8** throughout (the widened `30` broke phase 2 in slice 8). The
pre-existing phase-11b module-stash flake did not recur in this wave's legs.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review, rust-port

## Next slice

**Family:** the **passcode smoker** (`securitycraft:keypad_smoker`), the remaining member of the
gap line *"the passcode chest/barrel/**furnace/smoker**, keypad door and trapdoor"* now that the
furnace is proved. It fits tier **deep** in the same logic+machines shape: the jar's 16-variant
`facing`/`lit` blockstate, its `keypad_smoker_*` models (which parent to the carried
`base_keypad_furnace_*` meshes) and its `keypad_smoker_window` texture, a second block id with the
same three-slot input/fuel/output container, the same `container_access`/`furnace_tick` rules and
the same harness commands; only the recipe type changes (`minecraft:smoking` instead of
`minecraft:smelting`, via `FurnaceKind::Smoker::recipe_type()`), so the same pure rule with a
different kind and a shorter cook time. No new renderer or hook kind.

**In-game assertion that would prove it:** place a passcode smoker beside the player with an
adjacent vanilla smoker as the no-code differential; `cset` a code as owner, confirm a wrong
`copen` leaves `open=0`, `screen=playing` and zero container slots; the right code must open three
slots, then `item replace block … container.0 with minecraft:raw_iron` and `container.1 with
minecraft:coal`, tick ~120 ticks (smoking is half the smelting time) and read slot 302 back as
`minecraft:iron_ingot x1` with `lit=true` and the input consumed, surviving a relaunch.
`SELFTEST=smoker` must invert the wrong-code/no-slots assertion and end `EXIT:1`. If the jar's
smoker model turns out to need a renderer, stop with `blocked(<symbol>)` instead of faking it.

The **keypad door and trapdoor**, the universal block modifier and the reinforced-block family
stay behind the smoker; the chest remains `blocked(register_block_entity_renderers)` until the pin
stamps a block-entity renderer surface.

## Slice 12 — the passcode smoker (2026-11-24, SDK `0a875da`) — 0.2.12

**Slice and gap line.** The brief's own line, from slice 11's `## Next slice` and the port's gap
list: *"the passcode chest/barrel/**furnace/smoker**, keypad door and trapdoor"*. The chest stays
`blocked(register_block_entity_renderers)` (slice 10, measured — `builtin/entity` plus two Java
renderers, no pin surface); this slice takes the **passcode smoker**
(`securitycraft:keypad_smoker`). Class **content**, tier **deep** in the ledger; the delivered
shape is the port's existing **logic + machines** half (saved owner/code, a three-slot
block-backed container, a one-shot code gate, a cook tick) plus one carried block — no renderer,
no new hook kind, no re-tiering. The furnace/smoker gap line is now closed.

### Inventory gate first (the brief's stop condition)

From the jar (`securitycraft-1.10.2.1.jar`, SHA-1 `8760079e…3251`), every smoker block and item
model was scanned for `builtin/entity`: all **seven** were `CLEAN` —
`keypad_smoker_{closed,closed_lit,open,open_lit}`, `keypad_smoker` (item), and the two
`smoker_mine` models (evidence: `slice10-furnace-inventory.log` extended by this slice's
`slice11-smoker-inventory.log`). The visible machine is a static JSON mesh: the four
`keypad_smoker_*` block models parent the already-carried
`base_keypad_furnace_{closed,open}` meshes and set `#furnace` to the vanilla
`block/smoker_front{,_on}` and `#window` to the jar's `keypad_smoker_window.png`
(365 B, SHA-256 `7baa43e86255cb1786070ac59dbf8d282db05ad9cfab03bde5a460c7e272cf32`,
extracted byte-for-byte). Both vanilla smoker-front textures are present in the prepared 26.3
root. **No renderer is needed**, so the brief's `blocked(<symbol>)` stop condition does not fire.

The jar's 16-variant blockstate (`facing` × `lit` × `open`) is trimmed to 26.3's **eight**
`facing` × `lit` variants: the prepared `assets/minecraft/blockstates/smoker.json` has exactly
`facing` and `lit`, no `open`. The open-lid models are carried but unreferenced, as for the
furnace (recorded as a deviation).

### Cook time and output — read from the artefact, not the brief

The brief proposed `raw_iron` + coal → `iron_ingot` and a 120-tick cook. The artefact disagrees,
and the artefact wins:

- `FurnaceKind::Smoker::recipe_type()` is **`minecraft:smoking`** (read from the pin's
  `horizon-client-render/src/containers.rs:71`), and `Recipes::cook` filters strictly by recipe
  type — there is no smelting fallback.
- The pack's **nine** `minecraft:smoking` recipes are all food (potato, beef, cod, chicken,
  mutton, rabbit, porkchop, salmon, kelp); each has `cookingtime: 200` and one output item.
  **There is no `iron_ingot_from_smoking` recipe** — raw iron can only be smelted or blasted.
  A real vanilla smoker cannot cook raw iron either, so the brief's proposed assertion cannot be
  faithful.
- The e2e therefore proves the recipe kind with a pair: `raw_iron` + coal leaves the output
  **empty** (the smoker is not using `minecraft:smelting`), then `minecraft:beef` + coal yields
  **`minecraft:cooked_beef` ×1** after 200 ticks with `lit=true` and the input consumed. Both
  the cook time and the output count come from the game data, not the brief. `SELFTEST=smoker`
  inverts the wrong-code/no-slots assertion and ends `EXIT:1`.

### Red proof (TDD, before the rule existed)

`src/runtime.rs` gained `CookKind { Furnace, Smoker }` with `recipe_type` stubbed to
`minecraft:smelting` for both arms, and `a_smoker_cooks_against_the_smoking_recipe_table` was
added first. A forwarded `cargo test --offline` (`slice11-red.log`) failed:

```
---- runtime::tests::a_smoker_cooks_against_the_smoking_recipe_table stdout ----

thread 'runtime::tests::a_smoker_cooks_against_the_smoking_recipe_table' panicked at src/runtime.rs:826:9:
assertion `left == right` failed
  left: "minecraft:smelting"
 right: "minecraft:smoking"

test result: FAILED. 59 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

The rule became `Furnace → minecraft:smelting`, `Smoker → minecraft:smoking`; the pure
`furnace_tick`, `FurnaceState` and `container_access` rules are reused unchanged. A glue test
`the_smoker_container_uses_a_three_slot_mod_menu` pins the new menu, giving **61** tests.

### What the slice implements

- **The rule** (`runtime::CookKind::recipe_type`): the only genuinely new pure rule — a smoker
  matches the smoking table. The burn/cook/consume/smelt timer and the owner-then-digest access
  verdict are the 0.2.11 rules, byte-shared with the furnace.
- **The glue** (`support.rs`): `cooking(host, level, stack, CookKind)` caches one `Identifier` per
  recipe type and calls `LevelRef::recipe_for`; `on_cooker_tick` is the shared tick and
  `on_furnace_tick`/`on_smoker_tick` pass the kind. The one-shot open gate, harness commands and
  menu-close spending are the barrel/furnace paths, with `cset`/`copen`/`cinfo` now labelling
  `securitycraft smoker …` by block id.
- **Content**: `securitycraft:keypad_smoker` (the jar's block/item ids, eight-state blockstate,
  four block models + item model, the window texture, simplified loot, lang and creative-tab
  entry, pickaxe tag), a three-slot `securitycraft:keypad_smoker` menu and a drawn input/fuel/
  output panel.
- **Parts** (`lib.rs`): `SmokerItem` records the placer as owner, `SmokerBlock` gates the click,
  `SmokerEntity` owns the block entity, tick and screen; registered as its own block-entity type.

### Verification (0.2.12, SDK `0a875da`, 2026-11-24)

Every Rust step ran through **forwarded** `build-slot.sh` leases on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`, relative `--port ports/securitycraft`,
`CARGO_INCREMENTAL=0`, **no `BUILD_LOCAL=1`**); the launcher build and the game legs stayed local
(the e2e is `--offscreen`). Evidence under `../_shared/evidence/securitycraft/` and
`../_shared/evidence/securitycraft-e2e/`.

| Check | Evidence | Result |
|---|---|---|
| Inventory gate | `slice11-smoker-inventory.log` | seven smoker models `CLEAN`; 26.3 smoker = `facing` × `lit` |
| Red | `slice11-red.log` | **59 passed / 1 failed**, `left: "minecraft:smelting"` / `right: "minecraft:smoking"` |
| Tests | `slice11-test-lint.log`, `sdk-test.log` (regenerated for 0.2.12) | **61 passed**, 0 failed |
| Format + lints | `slice11-test-lint.log` | fmt clean, clippy exit 0, **0 warnings** |
| Two builds + `cmp` | `slice11-build.log` | **byte-identical**; `rm -f Cargo.lock` before each |
| Delivered | `securitycraft.hmod` | **100,774 B**, SHA-256 `c71eefcf6df34528c2fedc5d9fd86fa15915b31339f0d3712a3a2274f11351d4`, **106 entries**, no `Cargo.lock`, version 0.2.12 |
| Registry check | `slice11-check-fits.log` | `securitycraft 0.2.12 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 |
| `fits --build 0a875da` | `slice11-check-fits.log` | `0a875da  made for`, EXIT 0 |
| `fits --build 0b14f8a` | `slice11-check-fits.log` | `0b14f8a  fits`, EXIT 0, same SHA |
| Gate | `slice11-verify-sdk-package.log` | **1/1**, `package matches the tree`, SDK-only deps, no lock, 61 tests |
| Launcher build | `slice11-launcher-build.log` | `Compiling securitycraft (1 of 1)`, `native-0a875da-75869b29741c built` |
| In-game PASS | `securitycraft-e2e/e2e-launcher.log` | **PASS + `EXIT:0`**, all 21 phases incl. phase 21 |
| Control (unmodified Nightly) | `e2e-control.log` | **`EXIT:1`** at phase 1 |
| Ids-unknown control | `e2e-ids-unknown.log` | PASS + `EXIT:0`, both ids unknown |
| SELFTEST (smoker) | `e2e-selftest-smoker.log` | **`EXIT:1`** at phase 21, "expected a wrong code to leave the smoker's contents unreadable" |
| Prior SELFTESTs (`1`/harming/nonloot/owner/relaunch/allow/deny/smart/disguise/container/furnace) | `e2e-selftest*.log` | **`EXIT:1`** at each own assertion |
| Log checker | `slice11-check-e2e-logs.log` | `logs=22 ok=22`, exit 0 |
| Recipe types | `slice11-check-recipe-types.log` | `securitycraft: ok`, exit 0 |

Phase 21 in the PASS leg (`e2e-launcher.log`):

```
unprotected vanilla smoker container slots: 3
coded smoker: securitycraft smoker 22 101 8 code=set open=0 owner=set lit=false
wrong-code smoker click: screen=playing container slots=0
after the right smoker code: securitycraft smoker 22 101 8 code=set open=1 owner=set lit=false
unlocked smoker container slots: 3
smoker output after raw iron + coal: empty
smoker output slot 302: minecraft:cooked_beef x1
smoker state while burning: securitycraft:keypad_smoker[facing=west,lit=true]
smoker input slot 300 after the cook: empty
smoker output slot 302 after the relaunch: minecraft:cooked_beef
```

The wrong-code pair (0 slots, `screen=playing`) is fail-capable: `SELFTEST=smoker` inverts it and
fails at `EXIT:1`. The vanilla smoker at the same spot opens with no code (differential), and the
raw-iron no-output is the recipe-kind differential that would fail if the smoker used smelting.
`ctl messages` stays at **8** throughout. Three pre-existing placement flakes stopped intermediate
legs before their own assertions (phase 14 twice, phase 17 twice); each identical leg was retried
unchanged and the flake logs are kept (`e2e-smoker-first-attempt-flake.log`,
`e2e-smoker-flake-17.log`, `e2e-smoker-selftest-flake-17.log`,
`e2e-furnace-selftest-flake-14.log`) — none is called green. A transient `ctl place` reach/occupancy
problem in the new phase was diagnosed: the target was 3.5 blocks from the player and held a
reinforced door from an earlier phase; phase 21 now clears its own spot and places within reach.

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:keypad_smoker` block/item (give, place, read back) | **tested** — phase 21: `block_name` and `block_state` |
| owner recorded on placement; first code saved | **tested** — phase 21: `owner=set`, `code=set` |
| wrong code → no permission, no menu, no slots | **tested** — phase 21: `open=0`, `screen=playing`, 0 slots; inverted by `SELFTEST=smoker` |
| right code → three-slot block-backed menu | **tested** — phase 21: `screen=container`, 3 slots |
| `minecraft:smoking` recipe kind (raw iron does not cook) | **tested** — phase 21: output stays `empty` |
| cook time and output count from the artefact (beef → cooked_beef ×1, 200 ticks) | **tested** — phase 21: slot 302 `minecraft:cooked_beef x1`, `lit=true`, input consumed |
| saved output and code survive a relaunch | **tested** — phase 21: slot 302 still cooked_beef after restart |
| unprotected vanilla smoker (differential) | **tested** — phase 21: 3 slots, no code |
| the smoker recipe-kind rule | **unit test** `a_smoker_cooks_against_the_smoking_recipe_table` |
| the three-slot smoker menu | **unit test** `the_smoker_container_uses_a_three_slot_mod_menu` |
| keypad chest / keypad door / trapdoor, universal block modifier, reinforced-block family | **not ported** — remaining gap list |
| the smoker's lid-open blockstate property | **not ported** — 26.3's smoker has only `facing`/`lit` (deviation) |
| modules installed on a smoker | **not ported** — modules install on keypads only |

### Deviations recorded by this slice

1. **The in-game cook uses a food recipe, not the brief's `raw_iron → iron_ingot`.** The pack's
   `minecraft:smoking` table is food-only (nine recipes, all `cookingtime: 200`, output ×1); raw
   iron has no smoking recipe in the artefact, so asserting iron would be unfaithful and would
   also fail on a vanilla smoker. The phase proves the recipe kind with raw iron's *no output*
   and the faithful output with beef. This is an artefact-over-brief ruling, recorded here.
2. **`open` blockstate property trimmed.** 26.3's smoker has only `facing` and `lit`; the jar's
   `base_keypad_furnace_open` models are carried but unreferenced. The menu opening is the
   observable "open".
3. **The loot table is simplified** to a survives-explosion drop of the smoker item, without the
   jar's `copy_components(custom_name)` transfer (as the barrel and furnace were).
4. **The code check is a harness command** (`cset`/`copen`/`cinfo`), not a drawn second screen, as
   for every other passcode family in this port.
5. **No faster cook multiplier.** The port's cook rule uses the matched recipe's `processing_time`
   unchanged; this pack's smoking recipes are all 200 ticks, so the smoker cooks at the same rate
   as the furnace here (the brief's "shorter cook time" is the vanilla 1.21.1 convention, not this
   pack's data). Recorded rather than applied as a made-up multiplier.

### Changelog for the resubmission (0.2.12)

> Supersedes #1805→0.2.11 (the staged PR). Carries the jar's `keypad_smoker` as a second cook-time
> machine beside the furnace: the same saved owner/code, three-slot container and one-shot code gate,
> ticked against the game's `minecraft:smoking` table via a new pure `CookKind::recipe_type` rule.
> The brief's raw-iron assertion was replaced from the artefact — the smoking table is food-only —
> and the in-game leg proves both that raw iron does not cook and that beef smokes into one
> cooked_beef with the input consumed and `lit=true`, surviving a relaunch. The salted SHA-256,
> SDK-only dependency set, save format, deny-before-allow order, smart gate and disguise render are
> unchanged.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review, rust-port

## Next slice

**Family:** the **keypad door and trapdoor** (`securitycraft:keypad_door`,
`securitycraft:keypad_trapdoor`), the next line of the gap list
(*"the passcode chest/barrel/furnace/smoker, **keypad door and trapdoor**"*) now that the
furnace/smoker pair is closed. It fits tier **deep** in the same logic+machines shape: a
passcode gate in front of a carried `minecraft:iron_door`/`minecraft:iron_trapdoor` template, the
same owner-and-code access rule, the same harness commands and the jar's blockstates/models.
Before dispatch, the next agent must inventory the door/trapdoor models for `builtin/entity`
and confirm the 26.3 door/trapdoor property names (`facing`, `half`, `hinge`, `open`, `powered`)
against the jar's variants, stopping with `blocked(<symbol>)` if a renderer is needed.

**In-game assertion that would prove it:** place a passcode door beside a keypad, set its code as
owner with `cset`, confirm a wrong `copen` leaves the door `open=false` and a right one flips it to
`open=true`; a vanilla iron door beside the player opens on redstone with no code (differential);
`SELFTEST=keypad-door` must invert the wrong-code/closed assertion and end `EXIT:1`. The
universal block modifier and the reinforced-block family stay behind it; the chest remains
`blocked(register_block_entity_renderers)`.

## Slice 13 — the keypad trapdoor (2026-11-24, SDK `0a875da`) — 0.2.13

**Slice and gap line.** The brief's own line, from slice 12's `## Next slice` and the port's gap
list: *"the passcode chest/barrel/furnace/smoker, **keypad door and trapdoor**"* now that the
furnace/smoker pair is closed. The chest stays `blocked(register_block_entity_renderers)` (slice 10,
measured); **do not retry it**. This slice takes the **keypad trapdoor**
(`securitycraft:keypad_trapdoor`). Class **content**, tier **deep** in the ledger; the delivered
shape is the port's existing **logic + machines** half (a block with saved owner/code, a code-gated
`open` flip and a closing tick) plus one carried block — no menu, renderer or new hook kind.

**Inventory gate first (the brief's stop condition).** Evidence:
[`slice12-keypad-door-inventory.log`](../../_shared/evidence/securitycraft/slice12-keypad-door-inventory.log).
Every door/trapdoor model in the jar (keypad, reinforced and scanner families, 37 files) was
scanned: **not one is `builtin/entity`** — all are static JSON meshes parented to vanilla
`block/door_*` / `block/template_trapdoor_*` models, and the three trapdoor parents exist in the
prepared 26.3 root. The jar's blockstates match 26.3 exactly: `keypad_door` 32 variants
(`facing`×`half`×`hinge`×`open`) = `iron_door`, `keypad_trapdoor` 16
(`facing`×`half`×`open`) = `iron_trapdoor`. Neither declares `powered`. **No renderer is needed,
no property is inexpressible → no `blocked` symbol.**

**Choice — the trapdoor first, because it is the cheaper expressible one.** 16 blockstate
variants against the door's 32, three static block models against eight, one block texture against
three, and a single block against a double-high two-block placement with `half`/`hinge`; the same
owner/code rule and harness commands cover both. The door is the next slice.

**The artefact's exact rule** (`KeypadTrapDoorBlock.java` / `KeypadTrapdoorBlockEntity.java`,
commit `c4d35b0c`, read for behaviour only): `useWithoutItem` answers `PASS` for an already-open
trapdoor with a signal length, checks `verifyPasscodeSet`, denies before allowing, then opens the
code screen; on a match `activate` cycles `OPEN`, plays the sound and schedules the off tick after
`signalLength` (default 60), and `tick` clears `OPEN`. `neighborChanged` is a **no-op**: the keypad
trapdoor never opens on redstone, while a vanilla iron trapdoor does — that is the differential.
`signalLength` is not configurable in this port (deviation below).

### Red proof (TDD, before the rule existed)

`src/runtime.rs` gained `TrapdoorChange`, `trapdoor_access` and `trapdoor_tick` with stubs that
kept the old behaviour, and two tests first. Forwarded `cargo test --offline`
([`slice12-red.log`](../../_shared/evidence/securitycraft/slice12-red.log)):

```
---- runtime::tests::a_right_code_flips_a_trapdoor_and_a_wrong_one_changes_nothing stdout ----
thread 'runtime::tests::a_right_code_flips_a_trapdoor_and_a_wrong_one_changes_nothing' panicked at src/runtime.rs:863:9:
assertion `left == right` failed: the owner's right code opens the trapdoor for signalLength ticks
  left: Unchanged
 right: Opened(60)

---- runtime::tests::a_trapdoor_closes_when_its_signal_length_elapses stdout ----
thread 'runtime::tests::a_trapdoor_closes_when_its_signal_length_elapses' panicked at src/runtime.rs:908:9:
assertion `left == right` failed
  left: (0, false)
 right: (1, false)

test result: FAILED. 61 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

The rules became `container_access` → `TrapdoorChange::Opened(signal_ticks)` only for the owner's
matching digest, and `trapdoor_tick = tick_signal`; all **64** tests pass.

### What the slice implements

- **Pure rules** (`runtime::TrapdoorChange`, `trapdoor_access`, `trapdoor_tick`): the owner-then-
  salted-SHA-256 verdict already proved for the containers, plus the artefact's flip and countdown.
- **Content**: `securitycraft:keypad_trapdoor` — the jar's block/item ids, byte-for-byte 16-state
  blockstate, three block models, item model and 532-byte texture, simplified loot, lang and
  creative-tab entry, pickaxe tag; a `minecraft:iron_trapdoor` template with the JAR's hardness,
  metal sounds and tool rule.
- **The gate** (`support::unlock_container`): `/securitycraft copen` calls the new rule; only a
  right code sets the saved countdown and `host.set_block_property(pos, "open", "true")`. A wrong
  code writes nothing, so `ctl block` still reads `open=false`. `/securitycraft cset` and `cinfo`
  dispatch by id; `cinfo` reports the block's own `open` state, parsed from the game, not the
  command's answer.
- **The close tick** (`support::on_trapdoor_tick`, a `BlockEntityPart` on its own block-entity
  type): when the saved 60 ticks elapse it clears `open` and the one-shot permission, as
  upstream's scheduled block tick does.
- **Right click** (`support::on_trapdoor_use`): an open trapdoor answers `PASS`; an iron trapdoor
  never opens by hand, so the carried template handles the rest (a headless harness cannot type
  into a second drawn panel — the established harness-command convention).

### Verification (0.2.13, SDK `0a875da`, 2026-11-24)

Every Rust step ran through **forwarded** `build-slot.sh` leases on `AdLf-PC`
(`build-slot: forwarding … to the remote runner`, relative `--port ports/securitycraft`,
`CARGO_INCREMENTAL=0`, **no `BUILD_LOCAL=1`**); the launcher build and the game legs stayed local
(the e2e is `--offscreen`). Evidence under `../_shared/evidence/securitycraft/` and
`../_shared/evidence/securitycraft-e2e/`.

| Check | Evidence | Result |
|---|---|---|
| Inventory gate | `slice12-keypad-door-inventory.log` | 37 models `CLEAN`; jar props = 26.3 props; no renderer |
| Red | `slice12-red.log` | **61 passed / 2 failed**, both new assertions (`Unchanged`/`Opened(60)`, `(0,false)`/`(1,false)`) |
| Tests | `slice12-test.log`, `sdk-test.log` (regenerated on the final tree) | **64 passed**, 0 failed |
| Format + lints | `slice12-lint.log`, `slice12-clippy.log` | fmt clean, clippy exit 0, **0 warnings** |
| Two builds + `cmp` | `slice12-build.log` | **byte-identical**; `rm -f Cargo.lock` before each |
| Delivered | `securitycraft.hmod` | **106,065 B**, SHA-256 `5f8857c08e6ab2168f8932b63512b2a39b7d7fa78c24770e8e97fc497ad74814`, **115 entries**, no `Cargo.lock`, version 0.2.13 |
| Registry check | `slice12-check-fits.log` | `securitycraft 0.2.13 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, same SHA, EXIT 0 |
| `fits --build 0a875da` | `slice12-check-fits.log` | `0a875da  made for`, EXIT 0 |
| `fits --build 0b14f8a` | `slice12-check-fits.log` | `0b14f8a  fits`, EXIT 0, same SHA |
| Gate | `slice12-verify-sdk-package.log` | **1/1**, `package matches the tree`, SDK-only deps (R9), no lock, 64 tests |
| Launcher build | `slice12-launcher-build.log` | `Compiling securitycraft (1 of 1)`, `native-0a875da-dc1e93ff95ab built` |
| In-game PASS | `securitycraft-e2e/e2e-launcher.log` | **PASS + `EXIT:0`**, all 22 phases incl. phase 22 |
| Control (unmodified Nightly) | `e2e-control.log` | **`EXIT:1`** at phase 1 |
| Ids-unknown control | `e2e-ids-unknown.log` | PASS + `EXIT:0`, ids unknown |
| SELFTEST (keypad trapdoor) | `e2e-selftest-keypad-trapdoor.log` | **`EXIT:1`** at phase 22, "expected the trapdoor OPEN after a wrong code" |
| SELFTEST (`keypad-door` alias) | `e2e-selftest-keypad-door.log` | **`EXIT:1`** at the same inverted phase-22 assertion |
| Prior SELFTEST (`1` door) | `e2e-selftest.log` | **`EXIT:1`** at phase 4, "expected the door OPEN after a wrong code" |
| Other prior SELFTESTs | `e2e-selftest*.log` | **`EXIT:1`** at each own assertion (logs kept green) |
| Log checker | `slice12-check-e2e-logs.log` | `logs=27 ok=27`, exit 0 |
| Recipe types | `slice12-check-recipe-types.log` | `securitycraft: ok`, exit 0 |

Phase 22 in the PASS leg (`e2e-launcher.log`):

```
vanilla iron trapdoor on redstone: minecraft:iron_trapdoor[facing=south,half=bottom,open=true,powered=true,waterlogged=false]
trapdoor after place: securitycraft:keypad_trapdoor[facing=west,half=bottom,open=false,powered=false,waterlogged=false]
securitycraft trapdoor 17 101 9 code=set open=false owner=set signal=0
trapdoor state after a wrong code: securitycraft:keypad_trapdoor[facing=west,half=bottom,open=false,powered=false,waterlogged=false]
trapdoor state after the right code: securitycraft:keypad_trapdoor[facing=west,half=bottom,open=true,powered=false,waterlogged=false]
trapdoor state after its signal: securitycraft:keypad_trapdoor[facing=west,half=bottom,open=false,powered=false,waterlogged=false]
PASS-trapdoor: the passcode trapdoor saved its owner and code, a wrong code left its block state open=false, the owner's right code flipped it to open=true, and its scheduled tick closed it again; a vanilla iron trapdoor beside it opened on redstone with no code
```

Every assertion is on the parsed `ctl block` state; `SELFTEST=keypad-trapdoor` and its
`keypad-door` alias invert the wrong-code/closed assertion and fail exactly there. `ctl messages`
stays at **8** throughout. Three pre-existing harness placement flakes (phase 17 twice, phase 14
twice) stopped intermediate legs before their own assertions; each identical leg was retried
unchanged and the flake logs are kept (`e2e-trapdoor-flake-17.log`, `e2e-trapdoor-flake-14.log`,
`e2e-selftest-keypad-door-flake-14.log`) — none is called green. The earlier transient syntax
typo in the new phase was fixed before these legs; its log is the same PASS evidence above.

### Coverage added by this slice

| Family | Status |
|---|---|
| `securitycraft:keypad_trapdoor` block/item (give, place, read back) | **tested** — phase 22: `block_name` and parsed `block_state` |
| owner recorded on placement, first code saved | **tested** — phase 22: `owner=set`, `code=set` |
| wrong code → `open=false` | **tested** — phase 22: parsed block state; inverted by both trapdoor SELFTESTs |
| right code → `open=true` | **tested** — phase 22: parsed block state, readout `open=true` |
| scheduled close after `signalLength` | **tested** — phase 22: `open=false` after 90 ticks |
| redstone differential (vanilla iron trapdoor opens, keypad trapdoor ignores redstone) | **tested** — phase 22: vanilla `open=true` with no code |
| artefact's no-op `neighborChanged` | **unit test** `a_right_code_flips_a_trapdoor_and_a_wrong_one_changes_nothing` + the PASS leg's unchanged state while redstone was present |
| keypad door, universal block modifier, reinforced-block family | **not ported** — next gap lines |
| keypad chest | **`blocked(register_block_entity_renderers)`** — slice 10, unchanged |

### Deviations recorded by this slice

1. **The code check is a harness command** (`cset`/`copen`/`cinfo`), as for the barrel, furnace and
   smoker; a headless harness cannot type into upstream's second drawn check panel.
2. **`signalLength` is fixed at the upstream default 60** and not configurable; the port's config
   file remains a gap. The saved countdown survives a relaunch (the tick is on the block entity).
3. **No `open` sound, disguise module, allowlist/denylist modules or `disabled` option** on the
   trapdoor; upstream's module inventory and disguise state on that block are a gap, as on the
   other carried containers.
4. **The item model is the jar's, the block declaration uses the `minecraft:iron_trapdoor`
   template**; its `powered`/`waterlogged` states are template states, not the mod's own (the jar's
   blockstate has none), and are never driven by the port.

### Changelog for the resubmission (0.2.13)

> Supersedes #1805→0.2.12 (the staged PR). Carries the jar's `keypad_trapdoor` (16-state
> blockstate, three static block models, texture and lang on a `minecraft:iron_trapdoor` template)
> as a code-gated trapdoor: the placer owns it, only the owner may set the first code, a wrong
> `copen` leaves its `open` state false, the right one flips it true, and a saved 60-tick countdown
> closes it — while a vanilla iron trapdoor opens on redstone with no code. New pure
> `trapdoor_access`/`trapdoor_tick` rules and their inverted in-game leg prove it. The salted
> SHA-256, SDK-only dependency set, save format, deny-before-allow order, smart gate and disguise
> render are unchanged.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-review, rust-port

## Next slice

**Family:** the **keypad door** (`securitycraft:keypad_door`), the other half of the same gap line
(*"…keypad door and trapdoor"*) now that the trapdoor is proved. It fits tier **deep** in the same
logic+machines shape: the jar's 32 `facing`/`half`/`hinge`/`open` blockstate matches 26.3's
`iron_door` exactly and all eight door models are static meshes (inventory gate evidence in
`slice12-keypad-door-inventory.log`), so the same `trapdoor_access`/`trapdoor_tick` rules, one-shot
gate and harness commands can drive a double-high carried door; only the block/item content and the
two-block placement differ. The cheap trapdoor was done first so the door reuses a proved path.

**In-game assertion that would prove it:** place a passcode door, `cset` its code as owner, confirm a
wrong `copen` leaves `open=false` and the right one flips both halves to `open=true` in the parsed
block state, then the scheduled tick closes it; a vanilla iron door beside the player opens on
redstone with no code (differential). `SELFTEST=keypad-door-full` must invert the wrong-code/closed
assertion and end `EXIT:1` (the existing `keypad-door` name is taken by the trapdoor alias). The
universal block modifier and the reinforced-block family stay behind it; the chest remains
`blocked(register_block_entity_renderers)` until the pin stamps a block-entity renderer surface.
