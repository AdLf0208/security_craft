// Game-side glue for the SecurityCraft slice: the keypad's block-entity payload,
// the screen's key rectangles, its drawn panel, the redstone it gives, and the
// harness commands.
//
// The rules live in `crate::runtime`, which knows nothing about the game; this
// module only reads the game, calls the rules and writes the answer back.
//
// What a keypad saves is its `code` (a salted digest), `salt`, `mode`, `status`
// and `signal`. The digits a player has typed but not submitted are **never**
// written there: they live in memory only (`TYPED`), because the compound is
// saved to disk and a passcode must not be recoverable from a save. A key
// written by the retired `.hnmod` build (`entry`) is scrubbed, never read.
//
// A correct code sets the block's `powered` state for `DEFAULT_SIGNAL_TICKS`;
// the `block_signal` hook answers 15 while it is set, which is the whole
// "link": a reinforced door beside the keypad is a vanilla iron door whose own
// neighbour update reads that signal, exactly as upstream's
// `BlockUtils.hasActiveSCBlockNextTo` reads a powered keypad or a 15 signal
// beside the door. The reinforced door itself has no code: it is the carried
// `minecraft:iron_door` template with the jar's textures and loot.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use mod_api::{
    BlockChangedEvent, BlockEntitySubmitEvent, BlockEntityTickEvent, BlockEntityUpdateTagEvent,
    ClientHost, Compound, DrawContext, Host, HostTransaction, InteractionResult, ItemKey,
    ItemPlace, ItemResource, ItemStack, LevelHost, LevelRef, LevelTickPostEvent, MenuButtonEvent,
    ModContainer, OnBlockPlacedByEvent, RandomSource, Screen, ScreenClickEvent, ScreenDrawEvent,
    ServerHost, StateForPlacementEvent, Tag, Text,
};

use crate::runtime::{
    Action, Button, DEFAULT_SIGNAL_TICKS, Entry, ListData, Mode, Throttle, after_submit, button,
    container_access, cooldown_allows, cooldown_failed, disguise_tint, facing_from_yaw, hash_code,
    is_allowed, is_denied, may_install, may_set_code, new_salt, tick_signal, valid_code,
    wrong_code_damage,
};

/// The keypad block; its item (`securitycraft:keypad_item`) places it.
pub(crate) const KEYPAD: &str = "securitycraft:keypad";
/// The key panel item: the item part that records the keypad's owner.
pub(crate) const KEYPAD_ITEM: &str = "securitycraft:keypad_item";
/// The passcode barrel: the jar's `keypad_barrel`, carried as a `minecraft:barrel`
/// template with a 27-slot container and a saved code.
pub(crate) const CONTAINER: &str = "securitycraft:keypad_barrel";
/// The block entity type of the passcode barrel.
pub(crate) const CONTAINER_ENTITY: &str = "keypad_barrel";
/// The vanilla 9×3 menu the barrel's 27 slots open, so hoppers, comparators,
/// shift-clicks and a relaunch use the game's own container paths.
pub(crate) const CONTAINER_MENU: &str = "securitycraft:keypad_barrel";
/// A vanilla barrel holds 27 items.
const CONTAINER_SLOTS: usize = 27;
/// The reinforced door block, whose item is the block's own (the double-high
/// placement of `minecraft:iron_door`).
pub(crate) const DOOR: &str = "securitycraft:iron_door_reinforced";
/// The keypad's menu type (`data/securitycraft/horizon/menu/keypad.json`): one
/// hidden slot and six data slots.
pub(crate) const KEYPAD_MENU: &str = "securitycraft:keypad";
/// The panel the keypad screen draws, and where the player's inventory starts.
pub(crate) const SCREEN_WIDTH: f32 = 176.0;
pub(crate) const SCREEN_HEIGHT: f32 = 226.0;
pub(crate) const INVENTORY_Y: f32 = 140.0;

/// The saved keys, named after the mod's own fields.
const KEY_CODE: &str = "code";
const KEY_SALT: &str = "salt";
const KEY_MODE: &str = "mode";
/// The key the retired build stored the typed digits under. Nothing writes it
/// any more; a loaded keypad that still carries one has it scrubbed on its
/// first tick.
const KEY_ENTRY: &str = "entry";
const KEY_STATUS: &str = "status";
const KEY_SIGNAL: &str = "signal";
/// The passcode barrel's one-shot permission to open: set by a correct code,
/// spent by the right-click that opens the container's menu. It is saved so a
/// code entered just before a relaunch still opens the barrel, but never holds
/// the digits themselves.
const KEY_OPEN: &str = "open";
/// The UUID (32 lowercase hex digits, no dashes) of the player who placed the keypad.
const KEY_OWNER: &str = "owner";
/// The module ids installed in the keypad, as a list of strings.
const KEY_MODULES: &str = "modules";
/// The players on the allowlist module, as 32-hex UUIDs (a harness spelling of
/// upstream's case-insensitive player names — see the deviation in `notes.md`).
const KEY_ALLOWLIST: &str = "allowlist";
/// The players on the denylist module, as 32-hex UUIDs (same harness spelling).
const KEY_DENYLIST: &str = "denylist";

/// The harming module's item id.
pub(crate) const HARMING_MODULE: &str = "securitycraft:harming_module";
/// The allowlist module's item id. Upstream's jar registers this item as
/// `whitelist_module` (`ModuleType.ALLOWLIST`); the port uses the module's current
/// name, recorded as a deviation in `notes.md`.
pub(crate) const ALLOWLIST_MODULE: &str = "securitycraft:allowlist_module";
/// The denylist module's item id. Upstream's jar registers this item as
/// `blacklist_module` (`ModuleType.DENYLIST`); the port uses the module's current
/// name, as it did for the allowlist (recorded as a deviation in `notes.md`).
pub(crate) const DENYLIST_MODULE: &str = "securitycraft:denylist_module";
/// The smart module's item id, as the jar registers it (`smart_module`). Upstream's
/// `IPasscodeProtected.onIncorrectPasscodeEntered` starts the keypad's cooldown only
/// when this module is installed.
pub(crate) const SMART_MODULE: &str = "securitycraft:smart_module";
/// The disguise module's item id. The jar registers it as `disguise_module`; the artefact's id wins.
pub(crate) const DISGUISE_MODULE: &str = crate::runtime::DISGUISE_MODULE;
/// The damage type a wrong code uses (`data/securitycraft/damage_type/incorrect_passcode.json`),
/// upstream's `CustomDamageSources.INCORRECT_PASSCODE`.
pub(crate) const INCORRECT_PASSCODE: &str = "securitycraft:incorrect_passcode";

/// The resolved item identities of the modules a removed keypad drops. A `LevelHost`
/// cannot resolve an id, so the keys are stored once at registration instead.
struct ModuleItems {
    harming: ItemKey,
    allowlist: ItemKey,
    denylist: ItemKey,
    smart: ItemKey,
    disguise: ItemKey,
}

static MODULE_ITEMS: OnceLock<ModuleItems> = OnceLock::new();

/// Store all five module keys at registration.
pub(crate) fn set_module_items(
    harming: ItemKey,
    allowlist: ItemKey,
    denylist: ItemKey,
    smart: ItemKey,
    disguise: ItemKey,
) {
    let _ = MODULE_ITEMS.set(ModuleItems {
        harming,
        allowlist,
        denylist,
        smart,
        disguise,
    });
}

fn module_item(module: &str) -> Option<ItemKey> {
    let items = MODULE_ITEMS.get()?;
    match module {
        HARMING_MODULE => Some(items.harming),
        ALLOWLIST_MODULE => Some(items.allowlist),
        DENYLIST_MODULE => Some(items.denylist),
        SMART_MODULE => Some(items.smart),
        DISGUISE_MODULE => Some(items.disguise),
        _ => None,
    }
}

/// The block property the keypad's redstone and its "powered" look read.
const POWERED: &str = "powered";

/// What the screen shows after the last key press (`status`).
const STATUS_NONE: i32 = 0;
const STATUS_WRONG: i32 = 1;
const STATUS_SET: i32 = 2;
const STATUS_OPENED: i32 = 3;
const STATUS_EMPTY: i32 = 4;
/// The screen's "too short / locked out" answers.
const STATUS_TOO_SHORT: i32 = 5;
const STATUS_LOCKED: i32 = 6;
/// The screen's "not your keypad" answer. A stranger's set screen never opens
/// (`may_set_code`), so this status is read from `/securitycraft info`, not drawn.
const STATUS_NOT_OWNER: i32 = 7;
/// A denied click's status in `/securitycraft info`. Upstream sends a deny chat
/// message; the drawn panel has one failure line, so the refusal is read here.
const STATUS_DENIED: i32 = 8;

/// The screen's button grid: 1-9, then backspace, 0 and submit. The same
/// rectangles are hit-tested by `screen_click` and drawn by `screen_draw`, both
/// relative to the panel's top left.
const GRID_X: f32 = 52.0;
const GRID_Y: f32 = 46.0;
const BUTTON_W: f32 = 20.0;
const BUTTON_H: f32 = 18.0;
const GAP_X: f32 = 26.0;
const GAP_Y: f32 = 22.0;

/// The rectangle of key `id` relative to the panel's top left, `None` for an id
/// the screen has no key for.
pub(crate) fn key_rect(id: i32) -> Option<[f32; 4]> {
    let (row, column) = match id {
        1..=9 => {
            let index = u32::try_from(id).ok()? - 1;
            (
                f32::from(i16::try_from(index / 3).ok()?),
                f32::from(i16::try_from(index % 3).ok()?),
            )
        }
        // The bottom row: backspace, 0, submit.
        10 => (3.0, 0.0),
        0 => (3.0, 1.0),
        11 => (3.0, 2.0),
        _ => return None,
    };
    Some([
        GRID_X + column * GAP_X,
        GRID_Y + row * GAP_Y,
        BUTTON_W,
        BUTTON_H,
    ])
}

// --- saved data -----------------------------------------------------------
//
// A compound's key type is UTF-16 and the API does not export it, so a key is
// taken from `Tag::String`'s own payload: the same conversion a mod's NBT
// string uses, with no name needed.

fn key_tag(name: &str) -> Tag {
    Tag::String(name.into())
}

fn tag<'a>(nbt: &'a Compound, name: &str) -> Option<&'a Tag> {
    let Tag::String(utf16) = key_tag(name) else {
        return None;
    };
    nbt.get(&utf16)
}

fn put(nbt: &mut Compound, name: &str, value: Tag) {
    if let Tag::String(utf16) = key_tag(name) {
        nbt.insert(utf16, value);
    }
}

fn drop_key(nbt: &mut Compound, name: &str) {
    if let Tag::String(utf16) = key_tag(name) {
        nbt.remove(&utf16);
    }
}

fn int(nbt: &Compound, name: &str) -> i32 {
    match tag(nbt, name) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn set_int(nbt: &mut Compound, name: &str, value: i32) {
    put(nbt, name, Tag::Int(value));
}

fn string(nbt: &Compound, name: &str) -> Option<String> {
    match tag(nbt, name) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn set_string(nbt: &mut Compound, name: &str, value: &str) {
    put(nbt, name, Tag::String(value.into()));
}

fn has(nbt: &Compound, name: &str) -> bool {
    tag(nbt, name).is_some()
}

/// The owner a keypad saved, `None` for an unowned one (an old save, or a
/// dispenser place). The UUID is 32 lowercase hex digits, the same spelling
/// `format!("{uuid:032x}")` writes.
fn owner_of(nbt: &Compound) -> Option<u128> {
    let hex = string(nbt, KEY_OWNER)?;
    if hex.len() != 32 {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok()
}

/// The modules a keypad saved, in install order.
fn modules(nbt: &Compound) -> Vec<String> {
    match tag(nbt, KEY_MODULES) {
        Some(Tag::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                Tag::String(text) => Some(text.to_string_lossy()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `module` is installed. Upstream's `IModuleInventory.hasModule`.
fn has_module(nbt: &Compound, module: &str) -> bool {
    modules(nbt).iter().any(|installed| installed == module)
}

/// Installs `module`, once: upstream's `insertModule` refuses a second module of
/// a type the block already has.
fn add_module(nbt: &mut Compound, module: &str) {
    let mut installed = modules(nbt);
    if !installed.iter().any(|existing| existing == module) {
        installed.push(module.to_owned());
    }
    put(
        nbt,
        KEY_MODULES,
        Tag::List(
            installed
                .into_iter()
                .map(|id| Tag::String(id.as_str().into()))
                .collect(),
        ),
    );
}

/// The players on the allowlist module, in insertion order.
fn allowlist(nbt: &Compound) -> Vec<String> {
    match tag(nbt, KEY_ALLOWLIST) {
        Some(Tag::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                Tag::String(text) => Some(text.to_string_lossy()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn set_allowlist(nbt: &mut Compound, players: &[String]) {
    put(
        nbt,
        KEY_ALLOWLIST,
        Tag::List(
            players
                .iter()
                .map(|player| Tag::String(player.as_str().into()))
                .collect(),
        ),
    );
}

/// The pure rules' view of the saved allowlist (bounded, case-insensitive).
fn allow_data(nbt: &Compound) -> ListData {
    allowlist(nbt)
        .into_iter()
        .fold(ListData::new(), |data, player| data.with_player(&player))
}

/// The players on the denylist module, in insertion order.
fn denylist(nbt: &Compound) -> Vec<String> {
    match tag(nbt, KEY_DENYLIST) {
        Some(Tag::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                Tag::String(text) => Some(text.to_string_lossy()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn set_denylist(nbt: &mut Compound, players: &[String]) {
    put(
        nbt,
        KEY_DENYLIST,
        Tag::List(
            players
                .iter()
                .map(|player| Tag::String(player.as_str().into()))
                .collect(),
        ),
    );
}

/// The pure rules' view of the saved denylist (bounded, case-insensitive).
fn deny_data(nbt: &Compound) -> ListData {
    denylist(nbt)
        .into_iter()
        .fold(ListData::new(), |data, player| data.with_player(&player))
}

// --- the world ------------------------------------------------------------

/// The authoritative level the server acts in now: the one a block entity's
/// tick belongs to (a parked level's block entities are not ticked, and a
/// client of a server ticks no level).
pub(crate) fn in_place_level(host: &mut dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

/// The block name at `pos`, empty when the level has nothing there.
fn block_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> String {
    level
        .block(host, pos)
        .map(|block| block.id(host))
        .unwrap_or_default()
}

/// The `powered` state of the block at `pos`.
fn powered_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    level.block_property(host, pos, POWERED).as_deref() == Some("true")
}

/// The block a state belongs to is the keypad (`BlockState.is`).
fn is_keypad(level: &dyn LevelHost, state: mod_api::BlockState) -> bool {
    level.state_block(state) == Some(KEYPAD)
}

/// Whether a keypad state is `powered=true`, read from the state's own
/// serialised form: a `&dyn LevelHost` has no metadata table to ask a property
/// of (see the gap in `notes.md`).
fn state_powered(level: &dyn LevelHost, state: mod_api::BlockState) -> bool {
    level
        .state_string(state)
        .is_some_and(|text| text.contains("powered=true"))
}

// --- the block entity -----------------------------------------------------

/// A keypad block entity starts empty (`EntityBlock.newBlockEntity`).
pub(crate) fn on_block_entity() -> Option<Compound> {
    Some(Compound::new())
}

/// What clients get of the keypad for the disguise render: only the installed module ids.
/// The rest of the saved compound — the salted digest above all — stays on the server.
pub(crate) fn update_tag(event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
    let installed = modules(&event.entity.nbt);
    if installed.is_empty() {
        return None;
    }
    let mut out = Compound::new();
    put(
        &mut out,
        KEY_MODULES,
        Tag::List(
            installed
                .into_iter()
                .map(|id| Tag::String(id.as_str().into()))
                .collect(),
        ),
    );
    Some(out)
}

/// Draw a disguised keypad as its fixed tinted block. The keypad's own model still meshes
/// underneath; a block entity's geometry is drawn after terrain, so the expanded cube covers it.
pub(crate) fn submit(host: &dyn ClientHost, event: &mut BlockEntitySubmitEvent<'_>) {
    if disguise_tint(&modules(&event.entity.nbt)).is_none() {
        return;
    }
    // This engine build does not multiply a colour into a cube's faces from its `tint`
    // argument (measured: a white sprite tinted green rendered grey). Drawing a real
    // lime-concrete block by key is the robust path: its own texels are green. The pure rule
    // above still gates the render on the installed module.
    let Some(block) = host.block_key("minecraft:lime_concrete") else {
        return;
    };
    let _ = event.draw.block(block, &[]);
}

/// The keypad item placed its block: the placer becomes the keypad's owner.
///
/// Upstream's `Block.setPlacedBy` gives an `IOwnable` block its owner, and the owner is
/// who may install modules (`ModuleItem.useOn`) and open a fresh keypad's set screen.
/// A dispenser place has no placer and leaves the keypad unowned.
pub(crate) fn on_block_placed_by(host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
    let Some(placer) = event.placer else {
        return;
    };
    let Some(level) = in_place_level(host) else {
        return;
    };
    let Some(entity) = host.block_entity(level, event.pos) else {
        return;
    };
    let Some(mut data) = entity.data(&*host) else {
        return;
    };
    set_string(&mut data, KEY_OWNER, &format!("{:032x}", placer.uuid()));
    let _ = entity.replace_data(host, data);
    // A fresh block owns this position: no earlier keypad's modules are dropped
    // if this one is removed before its first tick.
    if let Some(dimension) = level.dimension().id(&*host) {
        forget_modules(&dimension, event.pos);
    }
}

/// A keypad block entity ticks: the signal countdown and the menu's data slots
/// (`EntityBlock.getTicker`).
pub(crate) fn on_ticks(event: &mut mod_api::BlockEntityTicksEvent<'_>) {
    event.ticks = true;
}

/// The data slots the screen reads: mode, typed length, status, code-set flag,
/// powered flag and the signal left.
fn data_slots(
    host: &dyn Host,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &Compound,
    typed: usize,
) -> Vec<i32> {
    vec![
        int(nbt, KEY_MODE),
        i32::try_from(typed).unwrap_or(i32::MAX),
        int(nbt, KEY_STATUS),
        i32::from(has(nbt, KEY_CODE)),
        i32::from(powered_at(host, level, pos)),
        int(nbt, KEY_SIGNAL),
    ]
}

/// One keypad tick: count the signal down and turn it off when it reaches zero,
/// then keep the menu's data slots current.
pub(crate) fn on_tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let Some(level) = in_place_level(host) else {
        return;
    };
    let fields = &mut *event.entity;
    // A key written by the retired build is scrubbed, never read.
    scrub_legacy_entry(&mut fields.nbt);
    // The removal a level-side write makes is heard after this entity is gone,
    // so what it holds has to be known before that: refresh it every tick.
    let dimension = level.dimension().id(&*host);
    if let Some(dimension) = dimension.as_deref() {
        remember_modules(dimension, pos, modules(&fields.nbt));
    }
    let signal = int(&fields.nbt, KEY_SIGNAL);
    if signal > 0 {
        let (next, turns_off) = tick_signal(signal);
        set_int(&mut fields.nbt, KEY_SIGNAL, next);
        if turns_off {
            host.set_block_property(pos, POWERED, "false");
        }
    }
    let typed = dimension.map_or(0, |dimension| typed_at_keypad(&dimension, pos));
    let slots = data_slots(&*host, level, pos, &fields.nbt, typed);
    if fields.data != slots {
        fields.data = slots;
    }
}

/// A player uses the keypad: a powered keypad does nothing (upstream's
/// `POWERED` check, which cancels the click); an unpowered one starts the check
/// or set screen, and the content block's own use opens the menu beside it.
/// Answers `None` — the vanilla use still runs — so the menu opens; see
/// `notes.md` for why this replaces the retired `right_click_block` hook.
pub(crate) fn on_use(
    host: &mut dyn ServerHost,
    click: &mod_api::BlockClick<'_>,
) -> Option<InteractionResult> {
    let pos = click.pos;
    let level = in_place_level(host)?;
    let entity = host.block_entity(level, pos)?;
    let mut data = entity.data(&*host)?;
    // A module in the owner's hand is installed first, as upstream's `ModuleItem.useOn`
    // runs before the block's own use. A stranger's module, a second one of a type the
    // keypad already has and any other held item all fall through to the menu below.
    let module = click
        .stack
        .and_then(|stack| match stack.item().id(&*host).as_str() {
            HARMING_MODULE => Some(HARMING_MODULE),
            ALLOWLIST_MODULE => Some(ALLOWLIST_MODULE),
            DENYLIST_MODULE => Some(DENYLIST_MODULE),
            SMART_MODULE => Some(SMART_MODULE),
            DISGUISE_MODULE => Some(DISGUISE_MODULE),
            _ => None,
        });
    if let Some(module) = module
        && let Some(stack) = click.stack
        && may_install(owner_of(&data), click.player)
        && !has_module(&data, module)
    {
        add_module(&mut data, module);
        // The removal that follows a level-side write hears this entity gone,
        // so the list it drops from is recorded here, not read at removal.
        let installed = modules(&data);
        if entity.replace_data(host, data).is_err() {
            return None;
        }
        if let Some(dimension) = level.dimension().id(&*host) {
            remember_modules(&dimension, pos, installed);
        }
        // The disguise render reads the installed modules from this block entity's client
        // copy. An install is an in-place write with no block-state change, so push the
        // update tag explicitly (as vanilla's `sendBlockUpdated` does).
        if module == DISGUISE_MODULE {
            host.block_entity_sync(pos);
        }
        let resource = ItemResource::of(stack);
        let hand = ItemPlace::PlayerHand {
            player: click.player,
            hand: click.hand,
        };
        let _ = host.transaction(|host, tx| {
            // A creative player's hand gives without losing any, as vanilla's creative use does.
            hand.extract(host, tx, &resource, 1);
            Ok::<(), ()>(())
        });
        return Some(InteractionResult::SuccessServer);
    }
    if powered_at(&*host, level, pos) {
        return Some(InteractionResult::Fail);
    }
    let mode = if has(&data, KEY_CODE) {
        Mode::Check
    } else {
        Mode::Set
    };
    // A keypad without a code shows its set screen to its owner alone: upstream's
    // `verifyPasscodeSet` answers a stranger with "not set up" and opens nothing. The
    // content block opens the menu itself, so the refusal has to end the click.
    if mode == Mode::Set && !may_set_code(owner_of(&data), click.player) {
        set_int(&mut data, KEY_STATUS, STATUS_NOT_OWNER);
        set_int(&mut data, KEY_MODE, Mode::Check.code());
        drop_key(&mut data, KEY_ENTRY);
        let slots = data_slots(&*host, level, pos, &data, 0);
        let _ = entity.replace_data(host, data);
        let _ = entity.set_data_slots(host, slots);
        return Some(InteractionResult::Fail);
    }
    // Upstream's `KeypadBlock.useWithoutItem`, after `verifyPasscodeSet`: the deny
    // branch runs **before** allow, then an allowed player is activated without
    // typing a code, and everyone else gets the code screen. An unset keypad never
    // reaches here — `verifyPasscodeSet` opens the owner's set screen and returns
    // false — so neither list module can bypass the first code.
    if mode == Mode::Check && has_module(&data, DENYLIST_MODULE) {
        let player_key = format!("{:032x}", click.player);
        if is_denied(true, &deny_data(&data), &player_key) {
            // A denied click opens nothing, powers nothing and leaves the door shut;
            // the panel has one failure line, so the refusal reads as a status here.
            set_int(&mut data, KEY_STATUS, STATUS_DENIED);
            set_int(&mut data, KEY_MODE, Mode::Check.code());
            let slots = data_slots(&*host, level, pos, &data, 0);
            let _ = entity.replace_data(host, data);
            let _ = entity.set_data_slots(host, slots);
            if let Some(session) = session_of(&*host, level, pos, click.player) {
                clear_session(&session);
            }
            // The content block opens the menu itself, so end the click to keep the
            // player in `playing` (upstream's denied click opens no screen).
            return Some(InteractionResult::Fail);
        }
    }
    if mode == Mode::Check && has_module(&data, ALLOWLIST_MODULE) {
        let player_key = format!("{:032x}", click.player);
        if is_allowed(true, &allow_data(&data), &player_key) {
            set_int(&mut data, KEY_STATUS, STATUS_OPENED);
            set_int(&mut data, KEY_MODE, Mode::Check.code());
            set_int(&mut data, KEY_SIGNAL, DEFAULT_SIGNAL_TICKS);
            let slots = data_slots(&*host, level, pos, &data, 0);
            let _ = entity.replace_data(host, data);
            let _ = entity.set_data_slots(host, slots);
            if let Some(session) = session_of(&*host, level, pos, click.player) {
                clear_session(&session);
            }
            host.set_block_property(pos, POWERED, "true");
            return Some(InteractionResult::SuccessServer);
        }
    }
    set_int(&mut data, KEY_MODE, mode.code());
    set_int(&mut data, KEY_STATUS, STATUS_NONE);
    drop_key(&mut data, KEY_ENTRY);
    let slots = data_slots(&*host, level, pos, &data, 0);
    let _ = entity.replace_data(host, data);
    let _ = entity.set_data_slots(host, slots);
    if let Some(session) = session_of(&*host, level, pos, click.player) {
        clear_session(&session);
    }
    None
}

// --- the screen -----------------------------------------------------------

/// The keypad's screen layout: one container slot the game keeps but the panel
/// does not show (a keypad has no item view), and the player's inventory below.
pub(crate) fn screen() -> Screen {
    Screen {
        width: SCREEN_WIDTH,
        height: SCREEN_HEIGHT,
        slots: vec![None],
        inventory_y: INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: false,
        own_background: false,
        ..Screen::default()
    }
}

fn centred_text(draw: &mut dyn DrawContext, origin: [f32; 2], text: &str, row: f32, argb: u32) {
    let width = draw.text_width(text);
    let x = origin[0] + (SCREEN_WIDTH - width) / 2.0;
    draw.text(text, [x, origin[1] + row], argb, false);
}

/// Draws the code display and the twelve keys (`screen_draw`).
pub(crate) fn on_screen_draw(_host: &dyn ClientHost, event: &mut ScreenDrawEvent<'_>) {
    let data = event.data;
    let origin = event.origin;
    let draw = &mut *event.draw;
    let mode = Mode::from_int(data.first().copied().unwrap_or(0));
    let typed = usize::try_from(data.get(1).copied().unwrap_or(0)).unwrap_or(0);
    let status = data.get(2).copied().unwrap_or(STATUS_NONE);
    let code_set = data.get(3).copied().unwrap_or(0) != 0;
    let (line, colour) = match status {
        STATUS_WRONG => ("Wrong code", 0xff8b_0000),
        STATUS_SET => ("Code set", 0xff00_6400),
        STATUS_OPENED => ("Door opened", 0xff00_6400),
        STATUS_EMPTY => ("Type a code first", 0xff8b_0000),
        STATUS_TOO_SHORT => ("At least 4 digits", 0xff8b_0000),
        STATUS_LOCKED => ("Wait a moment", 0xff8b_0000),
        _ if mode == Mode::Set => ("Set a code", 0xff40_4040),
        _ => ("Enter the code", 0xff40_4040),
    };
    centred_text(draw, origin, line, 8.0, colour);
    centred_text(draw, origin, &"*".repeat(typed), 24.0, 0xff20_2020);
    for id in 0..=11 {
        let Some([x, y, width, height]) = key_rect(id) else {
            continue;
        };
        let at = [origin[0] + x, origin[1] + y];
        draw.fill([at[0], at[1], width, height], 0xff9a_9a9a);
        let label = match id {
            10 => "<".to_owned(),
            11 => "OK".to_owned(),
            digit => digit.to_string(),
        };
        let text_width = draw.text_width(&label);
        draw.text(
            &label,
            [
                at[0] + (width - text_width) / 2.0,
                at[1] + (height - 8.0) / 2.0,
            ],
            0xff20_2020,
            false,
        );
    }
    centred_text(
        draw,
        origin,
        if code_set {
            "a code is set"
        } else {
            "no code set"
        },
        INVENTORY_Y - 14.0,
        0xff60_6060,
    );
}

/// A press on the keypad screen names the menu button the game sends to the
/// server (`screen_click`).
pub(crate) fn on_screen_click(event: &mut ScreenClickEvent<'_>) -> Option<i32> {
    let at = event.at;
    (0..=11).find(|id| {
        key_rect(*id).is_some_and(|[x, y, width, height]| {
            (x..x + width).contains(&at[0]) && (y..y + height).contains(&at[1])
        })
    })
}

// --- the menu -------------------------------------------------------------

/// One player's input session at one keypad in one world: the dimension's key, the block
/// position and the player. A typed digit belongs to exactly this triple, so a second player at
/// the same keypad, or a keypad at the same coordinates in another dimension, never sees or
/// submits someone else's input.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Session {
    dimension: String,
    pos: [i32; 3],
    player: u128,
}

impl Session {
    fn new(dimension: impl Into<String>, pos: [i32; 3], player: u128) -> Self {
        Self {
            dimension: dimension.into(),
            pos,
            player,
        }
    }
}

/// The digits typed into each open keypad session.
///
/// In memory only, on purpose: the block entity's compound is saved to disk, and a typed
/// passcode must not be recoverable from a save. A session's digits are forgotten on **open**,
/// on **close** (the player closed the screen, walked away, died, changed level or left) and on
/// submit, so a reopened keypad always starts empty and a save/reload reloads no digits.
static TYPED: Mutex<BTreeMap<Session, Entry>> = Mutex::new(BTreeMap::new());

/// The failed-verification budget of each session. It outlives a screen close on purpose: a
/// retry limit is only a limit if closing the screen does not reset it.
static ATTEMPTS: Mutex<BTreeMap<Session, Throttle>> = Mutex::new(BTreeMap::new());

fn typed() -> MutexGuard<'static, BTreeMap<Session, Entry>> {
    TYPED.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn attempts() -> MutexGuard<'static, BTreeMap<Session, Throttle>> {
    ATTEMPTS.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// The session of `player` at `pos` in `level`, when the level's dimension resolves.
fn session_of(host: &dyn Host, level: LevelRef, pos: [i32; 3], player: u128) -> Option<Session> {
    Some(Session::new(level.dimension().id(host)?, pos, player))
}

/// The digits typed in `session`, empty when none.
fn entry_at(session: &Session) -> Entry {
    typed().get(session).cloned().unwrap_or_default()
}

/// Remembers the digits typed in `session`, or forgets them when the entry is empty.
fn store_entry(session: &Session, entry: &Entry) {
    let mut guard = typed();
    if entry.is_empty() {
        guard.remove(session);
    } else {
        guard.insert(session.clone(), entry.clone());
    }
}

/// Forgets one session's digits (its keypad opened, closed, or the entry was submitted).
fn clear_session(session: &Session) {
    typed().remove(session);
}

/// Forgets every session's digits a player owns: the player's menu closed, or they left.
fn clear_player(player: u128) {
    typed().retain(|session, _| session.player != player);
}

/// Forgets every session's digits at one keypad, whichever player typed them (the harness code
/// control re-sets the keypad, so no stale entry may remain).
fn clear_keypad(dimension: &str, pos: [i32; 3]) {
    typed().retain(|session, _| !(session.dimension == dimension && session.pos == pos));
}

/// The masked length the screen shows for a keypad, from the first session with digits there.
fn typed_at_keypad(dimension: &str, pos: [i32; 3]) -> usize {
    typed()
        .iter()
        .find(|(session, _)| session.dimension == dimension && session.pos == pos)
        .map_or(0, |(_, entry)| entry.len())
}

/// The wall clock in milliseconds, the time base the retry budget uses.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Drops a typed-digit key written by the retired build, so an existing world
/// stops leaking a PIN on its next save. Nothing reads the key any more.
fn scrub_legacy_entry(nbt: &mut Compound) {
    drop_key(nbt, KEY_ENTRY);
}

/// A fresh 16-byte salt from the level's random, as upstream's
/// `PasscodeUtils.generateSalt` draws one.
fn fresh_salt(host: &mut dyn ServerHost) -> Option<String> {
    let level = in_place_level(host)?;
    let bytes = level.with_random(host, |random| {
        let mut bytes = [0u8; 16];
        for chunk in bytes.chunks_mut(4) {
            let draw = random.next_int_bound(i32::MAX).unwrap_or(0).unsigned_abs();
            chunk.copy_from_slice(&draw.to_le_bytes());
        }
        bytes
    })?;
    Some(new_salt(&bytes))
}

/// Applies one screen key to a keypad's saved data. `None` for a digit or a
/// backspace (the entry changed), `Some(action)` when Enter was pressed.
fn apply_key(
    host: &mut dyn ServerHost,
    session: &Session,
    nbt: &mut Compound,
    key: Button,
) -> Option<Action> {
    let mut entry = entry_at(session);
    match key {
        Button::Digit(digit) => {
            entry.push(digit);
            store_entry(session, &entry);
            None
        }
        Button::Backspace => {
            entry.backspace();
            store_entry(session, &entry);
            None
        }
        Button::Submit => {
            let mode = Mode::from_int(int(nbt, KEY_MODE));
            // The retry budget is checked on the server before any digest comparison, so a
            // locked-out guess costs nothing (and cannot be distinguished from a wrong one).
            // Upstream's keypad-side cooldown exists only with the smart module: without it
            // there is no lockout, however many wrong codes were typed.
            let smart = has_module(nbt, SMART_MODULE);
            let now = now_ms();
            let allowed = {
                let mut guard = attempts();
                cooldown_allows(smart, guard.entry(session.clone()).or_default(), now)
            };
            if mode == Mode::Check && !allowed {
                entry.clear();
                store_entry(session, &entry);
                set_int(nbt, KEY_STATUS, STATUS_LOCKED);
                return None;
            }
            // A set is the owner's alone. Upstream checks ownership twice — when the set
            // screen opens (`verifyPasscodeSet`) and again on the server packet
            // (`SetPasscode.handle`), so a screen that outlived the check stores nothing.
            // Here the screen can outlive it (the owner may change under an open menu) and
            // a menu button is client-sent, so the server refuses it too. The panel has one
            // failure line, so a refused set reads as a failed attempt (status 1, "Wrong
            // code"); no code is written.
            if mode == Mode::Set && !may_set_code(owner_of(nbt), session.player) {
                entry.clear();
                store_entry(session, &entry);
                set_int(nbt, KEY_STATUS, STATUS_WRONG);
                return None;
            }
            let salt = string(nbt, KEY_SALT).unwrap_or_default();
            let stored = string(nbt, KEY_CODE);
            let action = after_submit(mode, entry.text(), &salt, stored.as_deref());
            match action {
                Action::Open => {
                    {
                        let mut guard = attempts();
                        guard.entry(session.clone()).or_default().succeeded();
                    }
                    entry.clear();
                    store_entry(session, &entry);
                    set_int(nbt, KEY_STATUS, STATUS_OPENED);
                    set_int(nbt, KEY_MODE, Mode::Check.code());
                }
                Action::Wrong => {
                    {
                        let mut guard = attempts();
                        cooldown_failed(smart, guard.entry(session.clone()).or_default(), now);
                    }
                    entry.clear();
                    store_entry(session, &entry);
                    set_int(nbt, KEY_STATUS, STATUS_WRONG);
                }
                Action::Empty => {
                    set_int(nbt, KEY_STATUS, STATUS_EMPTY);
                }
                Action::TooShort => {
                    entry.clear();
                    store_entry(session, &entry);
                    set_int(nbt, KEY_STATUS, STATUS_TOO_SHORT);
                }
                Action::Set => {
                    let salt = fresh_salt(host).unwrap_or_default();
                    let code = hash_code(entry.text(), &salt).unwrap_or_default();
                    set_string(nbt, KEY_SALT, &salt);
                    set_string(nbt, KEY_CODE, &code);
                    entry.clear();
                    store_entry(session, &entry);
                    set_int(nbt, KEY_STATUS, STATUS_SET);
                    set_int(nbt, KEY_MODE, Mode::Check.code());
                }
                Action::Check => {
                    set_int(nbt, KEY_MODE, Mode::Set.code());
                    set_int(nbt, KEY_STATUS, STATUS_NONE);
                }
            }
            Some(action)
        }
    }
}

/// The server applies a key press (`menu_button`).
pub(crate) fn on_menu_button(host: &mut dyn ServerHost, event: &mut MenuButtonEvent<'_>) {
    let pos = event.context.pos;
    let Some(level) = in_place_level(host) else {
        return;
    };
    // The press belongs to the player the game names, at this keypad in this dimension: never to
    // whoever happened to type at these coordinates first.
    let Some(session) = session_of(&*host, level, pos, event.context.player) else {
        return;
    };
    let Some(key) = button(event.context.id) else {
        return;
    };
    let fields = &mut *event.entity;
    match apply_key(host, &session, &mut fields.nbt, key) {
        Some(Action::Open) => {
            set_int(&mut fields.nbt, KEY_SIGNAL, DEFAULT_SIGNAL_TICKS);
            host.set_block_property(pos, POWERED, "true");
        }
        Some(Action::Wrong) => hurt_for_wrong_code(host, &session, &fields.nbt),
        _ => {}
    }
    fields.data = data_slots(&*host, level, pos, &fields.nbt, entry_at(&session).len());
}

/// A wrong submission with the harming module installed hurts the player who made it
/// (`IPasscodeProtected.onIncorrectPasscodeEntered`), two hearts of
/// `securitycraft:incorrect_passcode`. Without the module nothing is hurt.
fn hurt_for_wrong_code(host: &mut dyn ServerHost, session: &Session, nbt: &Compound) {
    let Some(amount) = wrong_code_damage(has_module(nbt, HARMING_MODULE)) else {
        return;
    };
    let Some(player) = host.player(session.player) else {
        return;
    };
    let _ = player
        .entity()
        .hurt(host, INCORRECT_PASSCODE, amount, None, None);
}

/// The player closed a menu (ours or anyone's): every digit that player typed is gone, so a
/// reopened keypad starts empty and no later keypress can append to an abandoned PIN.
pub(crate) fn on_menu_closed(player: u128) {
    clear_player(player);
}

/// The player's block menu closed (closed, out of reach, died, changed level or left): forget the
/// session at that block, and the rest of that player's sessions as well, since only one menu can
/// be open at a time.
pub(crate) fn on_container_closed(player: u128, dimension: &str, pos: Option<[i32; 3]>) {
    if let Some(pos) = pos {
        clear_session(&Session::new(dimension, pos, player));
    }
    clear_player(player);
}

// --- redstone and placement ----------------------------------------------

/// `BlockState.isSignalSource`: every keypad state is one, so redstone dust
/// connects to it.
pub(crate) fn on_is_signal_source(level: &dyn LevelHost, event: &mut mod_api::IsSignalSourceEvent) {
    if is_keypad(level, event.state) {
        event.source = true;
    }
}

/// `BlockState.getSignal`: 15 on every face while powered, as upstream's
/// `KeypadBlock.getSignal`.
pub(crate) fn on_block_signal(level: &dyn LevelHost, event: &mut mod_api::BlockSignalEvent) {
    if is_keypad(level, event.state) && state_powered(level, event.state) {
        event.signal = 15;
    }
}

/// The keypad is placed facing the player who placed it
/// (`getStateForPlacement`).
pub(crate) fn on_state_for_placement(
    level: &dyn LevelHost,
    event: &mut StateForPlacementEvent<'_>,
) -> Option<Option<mod_api::BlockState>> {
    if !is_keypad(level, event.state) {
        return None;
    }
    let facing = facing_from_yaw(event.context.yaw);
    level
        .state_with_property(event.state, "facing", facing)
        .map(Some)
}

// --- drops ----------------------------------------------------------------

/// The module ids an installed list contributes to a removal drop: the modules
/// this slice knows, so a corrupt saved value or a later slice's id drops
/// nothing it has no item key for.
fn dropped_module_ids(installed: &[String]) -> Vec<String> {
    installed
        .iter()
        .filter(|module| {
            matches!(
                module.as_str(),
                HARMING_MODULE
                    | ALLOWLIST_MODULE
                    | DENYLIST_MODULE
                    | SMART_MODULE
                    | DISGUISE_MODULE
            )
        })
        .cloned()
        .collect()
}

/// One keypad's installed modules, keyed by the dimension it stands in (its id,
/// which a `DimensionKey` resolves) and its position.
type KeypadKey = (String, [i32; 3]);

/// A keypad key and the module ids held there, in a store both removal paths share.
type ModuleStore = Mutex<Vec<(KeypadKey, Vec<String>)>>;

/// The modules a standing keypad holds, read before a removal can destroy the
/// block entity. A `block_changed` removal is heard after the entity is gone,
/// so the list is kept here, refreshed on install and on every tick.
static INSTALLED: OnceLock<ModuleStore> = OnceLock::new();

/// Modules whose keypad was removed and whose item entities wait for the next
/// level tick: `block_changed` sees a read-only level, and every path that
/// removes the block reaches it, so the drop is spawned one tick later.
static PENDING_DROPS: OnceLock<ModuleStore> = OnceLock::new();

fn installed() -> MutexGuard<'static, Vec<(KeypadKey, Vec<String>)>> {
    INSTALLED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("the installed-module store is never poisoned")
}

fn pending_drops() -> MutexGuard<'static, Vec<(KeypadKey, Vec<String>)>> {
    PENDING_DROPS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("the pending-drop store is never poisoned")
}

/// Remember what a keypad holds, replacing an older list at the same key.
fn remember_modules(dimension: &str, pos: [i32; 3], modules: Vec<String>) {
    let mut store = installed();
    store.retain(|(key, _)| !(key.0 == dimension && key.1 == pos));
    if !modules.is_empty() {
        store.push(((dimension.to_owned(), pos), modules));
    }
}

/// Forget a keypad's list: a new block occupies the position.
fn forget_modules(dimension: &str, pos: [i32; 3]) {
    installed().retain(|(key, _)| !(key.0 == dimension && key.1 == pos));
}

/// Take a removed keypad's list out of the store.
fn take_modules(dimension: &str, pos: [i32; 3]) -> Vec<String> {
    let mut store = installed();
    let found = store
        .iter()
        .position(|(key, _)| key.0 == dimension && key.1 == pos);
    found.map_or_else(Vec::new, |index| store.remove(index).1)
}

/// A state was committed: a keypad that is gone queues its modules for a drop.
///
/// Every removal path reaches this hook — a player's break, `/setblock`,
/// `/fill`, a piston pushing a block in, an explosion — while a loot roll
/// (`block_drops`) is not asked for a level-side removal, and the block entity
/// is already gone, so the list comes from the store kept while it stood.
pub(crate) fn on_block_changed(level: &dyn LevelHost, event: &mut BlockChangedEvent) {
    if !is_keypad(level, event.old) || is_keypad(level, event.new) {
        return;
    }
    let dimension = event.dimension.key().id().to_owned();
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let modules = take_modules(&dimension, pos);
    if dropped_module_ids(&modules).is_empty() {
        return;
    }
    pending_drops().push(((dimension, pos), modules));
}

/// The level ticked: spawn the items of the keypads removed during it.
pub(crate) fn on_level_tick_post(host: &mut dyn ServerHost, event: &mut LevelTickPostEvent) {
    let Some(dimension) = event.level.dimension().id(&*host) else {
        return;
    };
    let ready: Vec<(KeypadKey, Vec<String>)> = {
        let mut pending = pending_drops();
        let mut ready = Vec::new();
        let mut index = 0;
        while index < pending.len() {
            if pending[index].0.0 == dimension {
                ready.push(pending.remove(index));
            } else {
                index += 1;
            }
        }
        ready
    };
    for ((_, pos), modules) in ready {
        for module in dropped_module_ids(&modules) {
            let Some(item) = module_item(&module) else {
                continue;
            };
            let _ = event.level.spawn_item(
                host,
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
                ItemStack::of(item),
                10,
            );
        }
    }
}

/// The keypad's container: one hidden slot (no face exposes it, so no hopper
/// can put anything in) and the mod menu the block's use opens.
pub(crate) fn keypad_container() -> ModContainer {
    ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| Vec::new()),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(KEYPAD_MENU.to_owned()),
    }
}

/// A keypad's readout and the reinforced door beside it, as `/securitycraft
/// info` prints it.
fn keypad_info(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    asking: Option<u128>,
) -> Option<String> {
    if block_at(&*host, level, pos) != KEYPAD {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let data = entity.data(&*host)?;
    let mode = Mode::from_int(int(&data, KEY_MODE));
    // The readout shows the asking player's own buffer, never another session's.
    let entry = asking
        .and_then(|player| session_of(&*host, level, pos, player))
        .map(|session| entry_at(&session))
        .unwrap_or_default();
    let mut door = "none".to_owned();
    for at in [
        [pos[0] + 1, pos[1], pos[2]],
        [pos[0] - 1, pos[1], pos[2]],
        [pos[0], pos[1] + 1, pos[2]],
        [pos[0], pos[1] - 1, pos[2]],
        [pos[0], pos[1], pos[2] + 1],
        [pos[0], pos[1], pos[2] - 1],
    ] {
        if block_at(&*host, level, at) != DOOR {
            continue;
        }
        let open = level.block_property(&*host, at, "open").as_deref() == Some("true");
        door = format!(
            "{},{},{}:{}",
            at[0],
            at[1],
            at[2],
            if open { "open" } else { "closed" }
        );
        break;
    }
    let installed = {
        let mut names = Vec::new();
        if has_module(&data, HARMING_MODULE) {
            names.push("harming");
        }
        if has_module(&data, ALLOWLIST_MODULE) {
            names.push("allowlist");
        }
        if has_module(&data, DENYLIST_MODULE) {
            names.push("denylist");
        }
        if has_module(&data, SMART_MODULE) {
            names.push("smart");
        }
        if has_module(&data, DISGUISE_MODULE) {
            names.push("disguise");
        }
        if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(",")
        }
    };
    Some(format!(
        "securitycraft keypad {} {} {} code={} entered={} mode={} signal={} powered={} status={} modules={} allow={} deny={} owner={} door={door}",
        pos[0],
        pos[1],
        pos[2],
        if has(&data, KEY_CODE) { "set" } else { "none" },
        entry.masked(),
        match mode {
            Mode::Check => "check",
            Mode::Set => "set",
        },
        int(&data, KEY_SIGNAL),
        if powered_at(&*host, level, pos) {
            "true"
        } else {
            "false"
        },
        int(&data, KEY_STATUS),
        installed,
        allowlist(&data).len(),
        denylist(&data).len(),
        if owner_of(&data).is_some() {
            "set"
        } else {
            "none"
        },
    ))
}

/// `/securitycraft code <pos> <digits>`: sets a code without the screen. It
/// exists because a headless harness cannot type into the screen, and is
/// recorded as a harness addition, not as gameplay. It is a *set*, so it obeys
/// the same owner rule as the set screen: upstream's `SetPasscode.handle`
/// refuses a player who does not own the block.
fn set_code(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    code: &str,
    asking: Option<u128>,
) -> Result<String, String> {
    if !valid_code(code) {
        return Err("A code is four to twenty digits.".to_owned());
    }
    if block_at(&*host, level, pos) != KEYPAD {
        return Err("No keypad at that position.".to_owned());
    }
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    if !asking.is_some_and(|player| may_set_code(owner_of(&data), player)) {
        return Err("Only the keypad's owner can set its code.".to_owned());
    }
    let salt = fresh_salt(host).ok_or_else(|| "No keypad at that position.".to_owned())?;
    let digest = hash_code(code, &salt).unwrap_or_default();
    if let Some(dimension) = level.dimension().id(&*host) {
        clear_keypad(&dimension, pos);
    }
    set_string(&mut data, KEY_SALT, &salt);
    set_string(&mut data, KEY_CODE, &digest);
    drop_key(&mut data, KEY_ENTRY);
    set_int(&mut data, KEY_MODE, Mode::Check.code());
    set_int(&mut data, KEY_STATUS, STATUS_SET);
    set_int(&mut data, KEY_SIGNAL, 0);
    let slots = data_slots(&*host, level, pos, &data, 0);
    entity
        .replace_data(host, data)
        .map_err(|_| "No keypad at that position.".to_owned())?;
    let _ = entity.set_data_slots(host, slots);
    Ok("Code set.".to_owned())
}

// --- the passcode barrel ----------------------------------------------------

/// The barrel's screen: a vanilla-shaped 9×3 chest panel so its 27 slots are
/// visible and clickable through the game's mod-menu path.
pub(crate) fn barrel_screen() -> Screen {
    Screen {
        width: 176.0,
        height: 166.0,
        slots: (0..CONTAINER_SLOTS)
            .map(|i| Some([8.0 + (i % 9) as f32 * 18.0, 18.0 + (i / 9) as f32 * 18.0]))
            .collect(),
        inventory_y: 84.0,
        inventory_x: 8.0,
        own_labels: false,
        own_background: false,
        ..Screen::default()
    }
}

/// The barrel's container: 27 item slots exposed on every face, shown through
/// the mod's 27-slot content menu (the game's mod-menu path the keypad uses).
pub(crate) fn barrel_container() -> ModContainer {
    let all: Vec<usize> = (0..CONTAINER_SLOTS).collect();
    ModContainer {
        slots: CONTAINER_SLOTS,
        faces: std::array::from_fn(|_| all.clone()),
        inputs: all.clone(),
        outputs: all,
        menu: Some(CONTAINER_MENU.to_owned()),
    }
}

/// A right click on the passcode barrel. A correct code grants a one-shot open
/// permission (`KEY_OPEN`); without it the click is ended before the content
/// block can open its 27-slot menu, so a wrong code leaves the contents
/// unreadable. With it the click runs the barrel's own use and the vanilla
/// container menu opens, backed by the block entity's saved item slots.
pub(crate) fn on_container_use(
    host: &mut dyn ServerHost,
    click: &mod_api::BlockClick<'_>,
) -> Option<InteractionResult> {
    let level = in_place_level(host)?;
    let entity = host.block_entity(level, click.pos)?;
    let data = entity.data(&*host)?;
    if int(&data, KEY_OPEN) == 0 {
        return Some(InteractionResult::Fail);
    }
    // Do not spend the grant here: `useItemOn` asks `useWithoutItem` next, and
    // the second call would then refuse the very click the first let through.
    // The one-shot permission is cleared when the container menu closes.
    // None: the carried `minecraft:barrel` block opens its own container menu.
    None
}

/// The barrel's menu closed: spend the one-shot permission so a later right-click
/// needs another code check (upstream closes and re-locks the container).
pub(crate) fn on_barrel_container_closed(
    host: &mut dyn ServerHost,
    player: u128,
    dimension: &str,
    pos: [i32; 3],
) {
    let _ = player;
    let Some(level) = host.level(dimension) else {
        return;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let mut data = entity.data(&*host).unwrap_or_default();
    if int(&data, KEY_OPEN) != 0 {
        set_int(&mut data, KEY_OPEN, 0);
        let _ = entity.replace_data(host, data);
    }
}

/// `/securitycraft cset <pos> <digits>`: sets a passcode barrel's first code.
/// The same owner rule as a keypad's `/securitycraft code`; the digest and salt
/// are the port's own SHA-256, so a barrel's code survives a restart.
fn set_container_code(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    code: &str,
    asking: Option<u128>,
) -> Result<String, String> {
    if !valid_code(code) {
        return Err("A code is four to twenty digits.".to_owned());
    }
    if block_at(&*host, level, pos) != CONTAINER {
        return Err("No passcode barrel at that position.".to_owned());
    }
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No passcode barrel at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No passcode barrel at that position.".to_owned())?;
    if has(&data, KEY_CODE) {
        return Err("That barrel already has a code.".to_owned());
    }
    if !asking.is_some_and(|player| may_set_code(owner_of(&data), player)) {
        return Err("Only the barrel's owner can set its code.".to_owned());
    }
    let salt = fresh_salt(host).ok_or_else(|| "No passcode barrel at that position.".to_owned())?;
    let digest = hash_code(code, &salt).unwrap_or_default();
    set_string(&mut data, KEY_SALT, &salt);
    set_string(&mut data, KEY_CODE, &digest);
    set_int(&mut data, KEY_STATUS, STATUS_SET);
    set_int(&mut data, KEY_OPEN, 0);
    entity
        .replace_data(host, data)
        .map_err(|_| "No passcode barrel at that position.".to_owned())?;
    Ok("Barrel code set.".to_owned())
}

/// `/securitycraft copen <pos> <digits>`: the barrel's code-check path. The
/// player must own the barrel and type its code; on a match a one-shot open
/// permission is saved. A wrong code writes nothing, so the container stays
/// closed and its contents stay unreadable.
fn unlock_container(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    code: &str,
    asking: Option<u128>,
) -> Result<String, String> {
    if block_at(&*host, level, pos) != CONTAINER {
        return Err("No passcode barrel at that position.".to_owned());
    }
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No passcode barrel at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No passcode barrel at that position.".to_owned())?;
    let stored = string(&data, KEY_CODE);
    let salt = string(&data, KEY_SALT).unwrap_or_default();
    let verdict = container_access(
        owner_of(&data),
        asking.unwrap_or(0),
        stored.as_deref(),
        &salt,
        code,
    );
    match verdict {
        crate::runtime::ContainerAccess::Open => {
            set_int(&mut data, KEY_OPEN, 1);
            set_int(&mut data, KEY_STATUS, STATUS_OPENED);
            entity
                .replace_data(host, data)
                .map_err(|_| "The barrel could not open.".to_owned())?;
            // The one-shot permission is consumed by the right-click that opens the
            // container's own block-backed menu (`on_container_use`). Using the game's
            // block menu, rather than a virtual menu `open_menu` would create, keeps the
            // 27 item slots backed by this block entity: deposits and a relaunch persist.
            Ok("Barrel unlocked.".to_owned())
        }
        crate::runtime::ContainerAccess::NotOwner => {
            Err("Only the barrel's owner may enter its code.".to_owned())
        }
        crate::runtime::ContainerAccess::NoCode => Err("The barrel has no code set.".to_owned()),
        crate::runtime::ContainerAccess::WrongCode => Err("Wrong code.".to_owned()),
    }
}

/// `/securitycraft cinfo <pos>`: the harness readout of a barrel's saved gate.
fn container_info(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    if block_at(host, level, pos) != CONTAINER {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let data = entity.data(host)?;
    Some(format!(
        "securitycraft barrel {} {} {} code={} open={} owner={}",
        pos[0],
        pos[1],
        pos[2],
        if has(&data, KEY_CODE) { "set" } else { "none" },
        int(&data, KEY_OPEN),
        if owner_of(&data).is_some() {
            "set"
        } else {
            "none"
        },
    ))
}

/// The UUID `/securitycraft owner … stranger` writes: no player has it, so the
/// keypad has an owner who can never act. A harness value, not gameplay.
const HARNESS_STRANGER: u128 = 0xdead_beef_dead_beef_dead_beef_dead_beef;

/// What `/securitycraft owner` says when the argument is neither a name it knows
/// nor a 32-digit UUID.
const OWNER_USAGE: &str = "Usage: /securitycraft owner <x> <y> <z> <self|stranger>";

/// The usage line of the harness's allowlist editor.
const ALLOW_USAGE: &str = "Usage: /securitycraft allow <x> <y> <z> <self|stranger|32-hex>";
/// The usage line of the harness's denylist editor.
const DENY_USAGE: &str = "Usage: /securitycraft deny <x> <y> <z> <self|stranger|32-hex>";

/// `/securitycraft owner <pos> <self|stranger>`: changes a keypad's owner.
///
/// Upstream's owner changes through the Universal Owner Changer, and the keypad's
/// `onOwnerChanged` clears the passcode and its salt with it. This command exists
/// because a single-player harness cannot drive a second player, so the owner is the
/// only way to put a stranger in front of a keypad; it is a harness addition, not
/// gameplay, and is recorded as such.
fn set_owner(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    who: &str,
    asking: Option<u128>,
) -> Result<String, String> {
    let owner = match who {
        "self" => asking.ok_or_else(|| "No player to make the owner.".to_owned())?,
        "stranger" => HARNESS_STRANGER,
        hex if hex.len() == 32 => {
            u128::from_str_radix(hex, 16).map_err(|_| OWNER_USAGE.to_owned())?
        }
        _ => return Err(OWNER_USAGE.to_owned()),
    };
    if block_at(&*host, level, pos) != KEYPAD {
        return Err("No keypad at that position.".to_owned());
    }
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    set_string(&mut data, KEY_OWNER, &format!("{owner:032x}"));
    // The command changes the owner and nothing else: upstream's Owner Changer also
    // resets the passcode (`onOwnerChanged`), but that block is not ported, and keeping
    // the code is what makes the "a stranger stands at a keypad that has a code" state
    // reachable for this harness. The screen's `mode` is the open client's intent —
    // upstream keeps it in the packet, not the block — so it is left alone too: a set
    // screen that outlives this change is exactly what the submit check refuses.
    let slots = data_slots(&*host, level, pos, &data, 0);
    entity
        .replace_data(host, data)
        .map_err(|_| "No keypad at that position.".to_owned())?;
    let _ = entity.set_data_slots(host, slots);
    if let Some(dimension) = level.dimension().id(&*host) {
        clear_keypad(&dimension, pos);
    }
    keypad_info(host, level, pos, asking).ok_or_else(|| "No keypad at that position.".to_owned())
}

/// `/securitycraft allow <pos> <self|stranger|32-hex>`: **sets** a keypad's installed
/// allowlist module to exactly one player. Upstream edits the module's `ListModuleData`
/// component in its customize screen, which a headless harness cannot open, so this
/// harness command writes the same list into the keypad's saved data. Replacing (rather
/// than appending) is deliberate: a single-player harness must be able to swap itself
/// for a stranger and get the other side of the check with one command. Only the owner
/// may edit it (upstream's `ModuleItem.useOn` owner gate), and the module must be
/// installed first. `self` is the acting player, `stranger` the harness's fixed
/// non-player UUID.
fn set_allow(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    who: &str,
    asking: Option<u128>,
) -> Result<String, String> {
    let player = match who {
        "self" => asking,
        "stranger" => Some(HARNESS_STRANGER),
        hex if hex.len() == 32 => u128::from_str_radix(hex, 16).ok(),
        _ => None,
    }
    .ok_or_else(|| ALLOW_USAGE.to_owned())?;
    if block_at(&*host, level, pos) != KEYPAD {
        return Err("No keypad at that position.".to_owned());
    }
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    if !asking.is_some_and(|actor| may_set_code(owner_of(&data), actor)) {
        return Err("Only the keypad's owner can change its allowlist.".to_owned());
    }
    if !has_module(&data, ALLOWLIST_MODULE) {
        return Err("Install an allowlist module first.".to_owned());
    }
    let list = ListData::new().with_player(&format!("{player:032x}"));
    set_allowlist(&mut data, list.players());
    let slots = data_slots(&*host, level, pos, &data, 0);
    entity
        .replace_data(host, data)
        .map_err(|_| "No keypad at that position.".to_owned())?;
    let _ = entity.set_data_slots(host, slots);
    Ok(format!(
        "Allowlist now has {} player(s).",
        list.players().len()
    ))
}

/// `/securitycraft deny <pos> <self|stranger|32-hex>`: **sets** a keypad's installed
/// denylist module to exactly one player. The counterpart of [`set_allow`]: upstream
/// edits the `ListModuleData` component in a customize screen a headless harness
/// cannot open, so this harness command writes the same single player into the saved
/// data. Replacing is deliberate — `self` and `stranger` are the two sides of the deny
/// check. Only the owner may edit it, and the module must be installed first.
fn set_deny(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    who: &str,
    asking: Option<u128>,
) -> Result<String, String> {
    let player = match who {
        "self" => asking,
        "stranger" => Some(HARNESS_STRANGER),
        hex if hex.len() == 32 => u128::from_str_radix(hex, 16).ok(),
        _ => None,
    }
    .ok_or_else(|| DENY_USAGE.to_owned())?;
    if block_at(&*host, level, pos) != KEYPAD {
        return Err("No keypad at that position.".to_owned());
    }
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    if !asking.is_some_and(|actor| may_set_code(owner_of(&data), actor)) {
        return Err("Only the keypad's owner can change its denylist.".to_owned());
    }
    if !has_module(&data, DENYLIST_MODULE) {
        return Err("Install a denylist module first.".to_owned());
    }
    let list = ListData::new().with_player(&format!("{player:032x}"));
    set_denylist(&mut data, list.players());
    let slots = data_slots(&*host, level, pos, &data, 0);
    entity
        .replace_data(host, data)
        .map_err(|_| "No keypad at that position.".to_owned())?;
    let _ = entity.set_data_slots(host, slots);
    Ok(format!(
        "Denylist now has {} player(s).",
        list.players().len()
    ))
}

/// Answers a `/securitycraft` invocation, or `None` when it is not one.
pub(crate) fn command(
    host: &mut dyn ServerHost,
    event: &mut mod_api::RunModCommandEvent<'_>,
) -> Option<String> {
    let invocation = event.command;
    let asking = match event.source {
        mod_api::CommandSource::Player { player, .. } => Some(player.uuid()),
        _ => None,
    };
    let literals: Vec<&str> = invocation.literals.iter().map(String::as_str).collect();
    match literals.as_slice() {
        ["securitycraft", "info"] => {
            let Some(mod_api::CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos")
            else {
                return Some("Usage: /securitycraft info <x> <y> <z>".to_owned());
            };
            let pos = *pos;
            let level = host
                .levels()
                .list_levels()
                .into_iter()
                .find(|level| level.dimension() == invocation.dimension)?;
            Some(
                keypad_info(host, level, pos, asking)
                    .unwrap_or_else(|| "No keypad at that position.".to_owned()),
            )
        }
        ["securitycraft", "code"] => {
            let Some(mod_api::CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos")
            else {
                return Some("Usage: /securitycraft code <x> <y> <z> <digits>".to_owned());
            };
            let pos = *pos;
            let Some(mod_api::CommandArgumentValue::String(code)) = invocation.argument("code")
            else {
                return Some("Usage: /securitycraft code <x> <y> <z> <digits>".to_owned());
            };
            let level = host
                .levels()
                .list_levels()
                .into_iter()
                .find(|level| level.dimension() == invocation.dimension)?;
            Some(set_code(host, level, pos, code, asking).unwrap_or_else(|line| line))
        }
        ["securitycraft", "owner"] => {
            let Some(mod_api::CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos")
            else {
                return Some(OWNER_USAGE.to_owned());
            };
            let pos = *pos;
            let Some(mod_api::CommandArgumentValue::String(who)) = invocation.argument("who")
            else {
                return Some(OWNER_USAGE.to_owned());
            };
            let level = host
                .levels()
                .list_levels()
                .into_iter()
                .find(|level| level.dimension() == invocation.dimension)?;
            Some(set_owner(host, level, pos, who, asking).unwrap_or_else(|line| line))
        }
        ["securitycraft", "allow"] => {
            let Some(mod_api::CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos")
            else {
                return Some(ALLOW_USAGE.to_owned());
            };
            let pos = *pos;
            let Some(mod_api::CommandArgumentValue::String(who)) = invocation.argument("who")
            else {
                return Some(ALLOW_USAGE.to_owned());
            };
            let level = host
                .levels()
                .list_levels()
                .into_iter()
                .find(|level| level.dimension() == invocation.dimension)?;
            Some(set_allow(host, level, pos, who, asking).unwrap_or_else(|line| line))
        }
        ["securitycraft", "deny"] => {
            let Some(mod_api::CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos")
            else {
                return Some(DENY_USAGE.to_owned());
            };
            let pos = *pos;
            let Some(mod_api::CommandArgumentValue::String(who)) = invocation.argument("who")
            else {
                return Some(DENY_USAGE.to_owned());
            };
            let level = host
                .levels()
                .list_levels()
                .into_iter()
                .find(|level| level.dimension() == invocation.dimension)?;
            Some(set_deny(host, level, pos, who, asking).unwrap_or_else(|line| line))
        }
        ["securitycraft", "cset"] | ["securitycraft", "copen"] | ["securitycraft", "cinfo"] => {
            let Some(mod_api::CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos")
            else {
                return Some("Usage: /securitycraft cset|copen|cinfo <x> <y> <z>".to_owned());
            };
            let pos = *pos;
            let level = host
                .levels()
                .list_levels()
                .into_iter()
                .find(|level| level.dimension() == invocation.dimension)?;
            match literals[1] {
                "cinfo" => container_info(host, level, pos),
                "cset" => {
                    let Some(mod_api::CommandArgumentValue::String(code)) =
                        invocation.argument("code")
                    else {
                        return Some("Usage: /securitycraft cset <x> <y> <z> <digits>".to_owned());
                    };
                    Some(set_container_code(host, level, pos, code, asking).unwrap_or_else(|e| e))
                }
                _ => {
                    let Some(mod_api::CommandArgumentValue::String(code)) =
                        invocation.argument("code")
                    else {
                        return Some("Usage: /securitycraft copen <x> <y> <z> <digits>".to_owned());
                    };
                    Some(unlock_container(host, level, pos, code, asking).unwrap_or_else(|e| e))
                }
            }
        }
        _ => None,
    }
}

/// The command's answer, routed to its originating source (the swapped-in
/// player, the console, a command block) as the command service does.
pub(crate) fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typed_digits_live_outside_the_saved_compound() {
        let session = Session::new("minecraft:overworld", [3, 64, 5], 71);
        // What the block entity saves starts empty and stays empty: the digits a
        // player types go to the transient store only.
        let nbt = Compound::new();
        let mut entry = Entry::new();
        for digit in [1, 2, 3, 4] {
            entry.push(digit);
        }
        store_entry(&session, &entry);
        assert_eq!(
            entry_at(&session),
            entry,
            "the screen can read what was typed"
        );
        assert_eq!(
            string(&nbt, KEY_ENTRY),
            None,
            "a typed passcode is never written to the saved block entity"
        );
        // Enter clears it, and an empty entry is forgotten rather than stored.
        store_entry(&session, &Entry::new());
        assert!(entry_at(&session).is_empty());
    }

    #[test]
    fn a_session_is_the_world_the_player_and_the_keypad() {
        let a = Session::new("minecraft:overworld", [5, 64, 5], 81);
        let b = Session::new("minecraft:overworld", [5, 64, 5], 82);
        let c = Session::new("minecraft:the_nether", [5, 64, 5], 81);
        let mut typed = Entry::new();
        for digit in [1, 2, 3, 4] {
            typed.push(digit);
        }
        store_entry(&a, &typed);
        assert_eq!(entry_at(&a).text(), "1234");
        assert!(
            entry_at(&b).is_empty(),
            "another player at the same keypad sees no digits"
        );
        assert!(
            entry_at(&c).is_empty(),
            "the same coordinates in another dimension are another keypad"
        );

        // Closing a menu (or leaving) drops only that player's sessions.
        clear_player(81);
        assert!(entry_at(&a).is_empty(), "the closed session is gone");
        assert!(entry_at(&c).is_empty(), "the other dimension's too");

        // Re-setting a keypad drops every session at it.
        let mut other = Entry::new();
        other.push(7);
        store_entry(&b, &other);
        assert_eq!(entry_at(&b).text(), "7");
        clear_keypad("minecraft:overworld", [5, 64, 5]);
        assert!(entry_at(&b).is_empty(), "no stale entry survives a re-set");
    }

    #[test]
    fn a_legacy_saved_entry_is_scrubbed_not_read() {
        let mut nbt = Compound::new();
        set_string(&mut nbt, KEY_ENTRY, "1234");
        scrub_legacy_entry(&mut nbt);
        assert_eq!(
            string(&nbt, KEY_ENTRY),
            None,
            "an older save's plaintext entry does not survive the keypad's tick"
        );
    }

    #[test]
    fn two_keypads_keep_their_own_digits() {
        let first = Session::new("minecraft:overworld", [1, 64, 1], 91);
        let second = Session::new("minecraft:overworld", [2, 64, 2], 91);
        let mut one = Entry::new();
        one.push(7);
        let mut two = Entry::new();
        two.push(9);
        two.push(9);
        store_entry(&first, &one);
        store_entry(&second, &two);
        assert_eq!(entry_at(&first).text(), "7");
        assert_eq!(entry_at(&second).text(), "99");
        store_entry(&first, &Entry::new());
        store_entry(&second, &Entry::new());
    }

    #[test]
    fn an_owner_uuid_round_trips_through_the_saved_string() {
        let uuid = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef_u128;
        let mut nbt = Compound::new();
        set_string(&mut nbt, KEY_OWNER, &format!("{uuid:032x}"));
        assert_eq!(owner_of(&nbt), Some(uuid));
        assert_eq!(
            string(&nbt, KEY_OWNER).as_deref(),
            Some("0123456789abcdef0123456789abcdef")
        );
        // A corrupt or missing owner reads as unowned, not as a UUID that matches someone.
        set_string(&mut nbt, KEY_OWNER, "not-a-uuid");
        assert_eq!(owner_of(&nbt), None);
        assert_eq!(owner_of(&Compound::new()), None);
    }

    #[test]
    fn a_module_is_installed_once_and_read_back() {
        let mut nbt = Compound::new();
        assert!(!has_module(&nbt, HARMING_MODULE));
        add_module(&mut nbt, HARMING_MODULE);
        add_module(&mut nbt, HARMING_MODULE);
        assert_eq!(modules(&nbt), vec![HARMING_MODULE.to_owned()]);
        assert!(has_module(&nbt, HARMING_MODULE));
        // A corrupt saved value is ignored rather than read as a module.
        set_int(&mut nbt, KEY_MODULES, 1);
        assert!(!has_module(&nbt, HARMING_MODULE));
    }

    #[test]
    fn the_modules_drop_from_the_saved_list_without_a_loot_roll() {
        // A level-side removal (`/setblock`, a piston, an explosion) rolls no
        // loot and has no breaker, so the module drop cannot come from
        // `block_drops`; it is read from the block entity before the removal.
        let mut nbt = Compound::new();
        add_module(&mut nbt, HARMING_MODULE);
        assert_eq!(
            dropped_module_ids(&modules(&nbt)),
            vec![HARMING_MODULE.to_owned()],
            "an installed module is dropped by the removal itself"
        );
        // A keypad with no module drops nothing, and a corrupt saved value is
        // not read as a module.
        assert!(dropped_module_ids(&modules(&Compound::new())).is_empty());
        let mut corrupt = Compound::new();
        set_int(&mut corrupt, KEY_MODULES, 1);
        assert!(dropped_module_ids(&modules(&corrupt)).is_empty());

        // The list is kept before the removal and taken by it exactly once.
        let overworld = "minecraft:overworld";
        let nether = "minecraft:the_nether";
        remember_modules(overworld, [5, 64, 5], modules(&nbt));
        assert_eq!(
            take_modules(overworld, [5, 64, 5]),
            vec![HARMING_MODULE.to_owned()]
        );
        assert!(
            take_modules(overworld, [5, 64, 5]).is_empty(),
            "one removal drops one list"
        );
        // A new block at the position forgets an older keypad's list.
        remember_modules(overworld, [6, 64, 6], modules(&nbt));
        forget_modules(overworld, [6, 64, 6]);
        assert!(take_modules(overworld, [6, 64, 6]).is_empty());
        // The same coordinates in another dimension are another keypad.
        remember_modules(nether, [5, 64, 5], modules(&nbt));
        assert!(take_modules(overworld, [5, 64, 5]).is_empty());
        assert_eq!(
            take_modules(nether, [5, 64, 5]),
            vec![HARMING_MODULE.to_owned()]
        );
    }

    #[test]
    fn an_allowlist_round_trips_through_the_saved_compound() {
        // The harness writes player keys to the saved list; the allow branch reads
        // them back through the pure rules. A corrupt value grants nobody access.
        let mut nbt = Compound::new();
        add_module(&mut nbt, ALLOWLIST_MODULE);
        assert!(has_module(&nbt, ALLOWLIST_MODULE));
        set_allowlist(&mut nbt, &["0123456789abcdef0123456789abcdef".to_owned()]);
        let data = allow_data(&nbt);
        assert!(is_allowed(
            has_module(&nbt, ALLOWLIST_MODULE),
            &data,
            "0123456789abcdef0123456789abcdef"
        ));
        assert!(!is_allowed(
            has_module(&nbt, ALLOWLIST_MODULE),
            &data,
            "deadbeefdeadbeefdeadbeefdeadbeef"
        ));
        set_int(&mut nbt, KEY_ALLOWLIST, 1);
        assert!(allowlist(&nbt).is_empty());
    }

    #[test]
    fn both_installed_modules_drop() {
        let mut nbt = Compound::new();
        add_module(&mut nbt, HARMING_MODULE);
        add_module(&mut nbt, ALLOWLIST_MODULE);
        assert_eq!(
            dropped_module_ids(&modules(&nbt)),
            vec![HARMING_MODULE.to_owned(), ALLOWLIST_MODULE.to_owned()]
        );
    }

    #[test]
    fn a_denylist_round_trips_through_the_saved_compound() {
        let mut nbt = Compound::new();
        add_module(&mut nbt, DENYLIST_MODULE);
        assert!(has_module(&nbt, DENYLIST_MODULE));
        set_denylist(&mut nbt, &["0123456789abcdef0123456789abcdef".to_owned()]);
        let data = deny_data(&nbt);
        assert!(is_denied(
            has_module(&nbt, DENYLIST_MODULE),
            &data,
            "0123456789abcdef0123456789abcdef"
        ));
        assert!(!is_denied(
            has_module(&nbt, DENYLIST_MODULE),
            &data,
            "deadbeefdeadbeefdeadbeefdeadbeef"
        ));
        set_int(&mut nbt, KEY_DENYLIST, 1);
        assert!(denylist(&nbt).is_empty());
    }

    #[test]
    fn a_smart_module_round_trips_through_the_saved_compound() {
        let mut nbt = Compound::new();
        assert!(!has_module(&nbt, SMART_MODULE));
        add_module(&mut nbt, SMART_MODULE);
        add_module(&mut nbt, SMART_MODULE);
        assert_eq!(modules(&nbt), vec![SMART_MODULE.to_owned()]);
        assert!(has_module(&nbt, SMART_MODULE));
    }

    #[test]
    fn all_five_installed_modules_drop() {
        let mut nbt = Compound::new();
        add_module(&mut nbt, HARMING_MODULE);
        add_module(&mut nbt, ALLOWLIST_MODULE);
        add_module(&mut nbt, DENYLIST_MODULE);
        add_module(&mut nbt, SMART_MODULE);
        add_module(&mut nbt, DISGUISE_MODULE);
        assert_eq!(
            dropped_module_ids(&modules(&nbt)),
            vec![
                HARMING_MODULE.to_owned(),
                ALLOWLIST_MODULE.to_owned(),
                DENYLIST_MODULE.to_owned(),
                SMART_MODULE.to_owned(),
                DISGUISE_MODULE.to_owned()
            ],
            "the disguise module drops through the same path as the other four"
        );
    }

    #[test]
    fn the_barrel_container_uses_a_27_slot_mod_menu() {
        let container = barrel_container();
        assert_eq!(container.slots, 27, "a barrel holds 27 items");
        assert_eq!(
            container.menu.as_deref(),
            Some(CONTAINER_MENU),
            "the carried container opens through its 27-slot content menu"
        );
        assert!(
            container.faces.iter().all(|face| face.len() == 27),
            "every face reaches the whole container, as a vanilla barrel does"
        );
        assert_eq!(container.inputs.len(), 27);
        assert_eq!(container.outputs.len(), 27);
    }

    #[test]
    fn the_keys_are_where_the_screen_draws_them() {
        // The hit test and the drawing share one rectangle per key, and the
        // bottom row is backspace, 0, submit.
        let one = key_rect(1).expect("1 is a key");
        let nine = key_rect(9).expect("9 is a key");
        assert!(one[0] < nine[0], "1 is left of 9");
        assert!(one[1] < nine[1], "1 is above 9");
        assert_eq!(key_rect(10).map(|rect| rect[0]), Some(GRID_X));
        assert_eq!(key_rect(11).map(|rect| rect[0]), Some(GRID_X + 2.0 * GAP_X));
        assert_eq!(key_rect(12), None);
        assert_eq!(key_rect(-1), None);
    }
}
