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
| `check-e2e-logs.py securitycraft --strict` | `logs=6 ok=6`, exit 0 |
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
