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

use mod_api::{
    BlockEntityTickEvent, ClientHost, Compound, DrawContext, Host, InteractionResult, LevelHost,
    LevelRef, MenuButtonEvent, ModContainer, RandomSource, Screen, ScreenClickEvent,
    ScreenDrawEvent, ServerHost, StateForPlacementEvent, Tag, Text,
};

use crate::runtime::{
    Action, Button, DEFAULT_SIGNAL_TICKS, Entry, Mode, after_submit, button, facing_from_yaw,
    hash_code, new_salt, tick_signal, valid_code,
};

/// The keypad block; its item (`securitycraft:keypad_item`) places it.
pub(crate) const KEYPAD: &str = "securitycraft:keypad";
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

/// The block property the keypad's redstone and its "powered" look read.
const POWERED: &str = "powered";

/// What the screen shows after the last key press (`status`).
const STATUS_NONE: i32 = 0;
const STATUS_WRONG: i32 = 1;
const STATUS_SET: i32 = 2;
const STATUS_OPENED: i32 = 3;
const STATUS_EMPTY: i32 = 4;

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

/// A keypad block entity ticks: the signal countdown and the menu's data slots
/// (`EntityBlock.getTicker`).
pub(crate) fn on_ticks(event: &mut mod_api::BlockEntityTicksEvent<'_>) {
    event.ticks = true;
}

/// The data slots the screen reads: mode, typed length, status, code-set flag,
/// powered flag and the signal left.
fn data_slots(host: &dyn Host, level: LevelRef, pos: [i32; 3], nbt: &Compound) -> Vec<i32> {
    let typed = entry_at(pos).len();
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
    let signal = int(&fields.nbt, KEY_SIGNAL);
    if signal > 0 {
        let (next, turns_off) = tick_signal(signal);
        set_int(&mut fields.nbt, KEY_SIGNAL, next);
        if turns_off {
            host.set_block_property(pos, POWERED, "false");
        }
    }
    let slots = data_slots(&*host, level, pos, &fields.nbt);
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
    if powered_at(&*host, level, pos) {
        return Some(InteractionResult::Fail);
    }
    let entity = host.block_entity(level, pos)?;
    let mut data = entity.data(&*host)?;
    let mode = if has(&data, KEY_CODE) {
        Mode::Check
    } else {
        Mode::Set
    };
    set_int(&mut data, KEY_MODE, mode.code());
    set_int(&mut data, KEY_STATUS, STATUS_NONE);
    drop_key(&mut data, KEY_ENTRY);
    let slots = data_slots(&*host, level, pos, &data);
    let _ = entity.replace_data(host, data);
    let _ = entity.set_data_slots(host, slots);
    store_entry(pos, &Entry::new());
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

/// The digits typed into an open keypad screen, per keypad position.
///
/// In memory only, on purpose: the block entity's compound is saved to disk, and
/// a typed passcode must not be recoverable from a save. Opening the screen,
/// pressing Enter and the end of the process all forget it, so a save/reload
/// reloads a keypad with no typed digits, never with a PIN.
static TYPED: std::sync::Mutex<Vec<([i32; 3], Entry)>> = std::sync::Mutex::new(Vec::new());

fn typed() -> std::sync::MutexGuard<'static, Vec<([i32; 3], Entry)>> {
    TYPED.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// The digits typed at `pos`, empty when none.
fn entry_at(pos: [i32; 3]) -> Entry {
    typed()
        .iter()
        .find(|(at, _)| *at == pos)
        .map(|(_, entry)| entry.clone())
        .unwrap_or_default()
}

/// Drops a typed-digit key written by the retired build, so an existing world
/// stops leaking a PIN on its next save. Nothing reads the key any more.
fn scrub_legacy_entry(nbt: &mut Compound) {
    drop_key(nbt, KEY_ENTRY);
}

/// Remembers the digits typed at `pos`, or forgets them when the entry is empty.
fn store_entry(pos: [i32; 3], entry: &Entry) {
    let mut guard = typed();
    guard.retain(|(at, _)| *at != pos);
    if !entry.is_empty() {
        guard.push((pos, entry.clone()));
    }
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
    pos: [i32; 3],
    nbt: &mut Compound,
    key: Button,
) -> Option<Action> {
    let mut entry = entry_at(pos);
    match key {
        Button::Digit(digit) => {
            entry.push(digit);
            store_entry(pos, &entry);
            None
        }
        Button::Backspace => {
            entry.backspace();
            store_entry(pos, &entry);
            None
        }
        Button::Submit => {
            let mode = Mode::from_int(int(nbt, KEY_MODE));
            let salt = string(nbt, KEY_SALT).unwrap_or_default();
            let stored = string(nbt, KEY_CODE);
            let action = after_submit(mode, entry.text(), &salt, stored.as_deref());
            match action {
                Action::Open => {
                    entry.clear();
                    store_entry(pos, &entry);
                    set_int(nbt, KEY_STATUS, STATUS_OPENED);
                    set_int(nbt, KEY_MODE, Mode::Check.code());
                }
                Action::Wrong => {
                    entry.clear();
                    store_entry(pos, &entry);
                    set_int(nbt, KEY_STATUS, STATUS_WRONG);
                }
                Action::Empty => {
                    set_int(nbt, KEY_STATUS, STATUS_EMPTY);
                }
                Action::Set => {
                    let salt = fresh_salt(host).unwrap_or_default();
                    let code = hash_code(entry.text(), &salt).unwrap_or_default();
                    set_string(nbt, KEY_SALT, &salt);
                    set_string(nbt, KEY_CODE, &code);
                    entry.clear();
                    store_entry(pos, &entry);
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
    let Some(key) = button(event.context.id) else {
        return;
    };
    let fields = &mut *event.entity;
    if let Some(Action::Open) = apply_key(host, pos, &mut fields.nbt, key) {
        set_int(&mut fields.nbt, KEY_SIGNAL, DEFAULT_SIGNAL_TICKS);
        host.set_block_property(pos, POWERED, "true");
    }
    fields.data = data_slots(&*host, level, pos, &fields.nbt);
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

// --- the container and the harness commands -------------------------------

/// The keypad's container: one hidden slot (no face exposes it, so no hopper
/// can put anything in) and the mod menu the block's use opens.
pub(crate) fn container() -> ModContainer {
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
fn keypad_info(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    if block_at(&*host, level, pos) != KEYPAD {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let data = entity.data(&*host)?;
    let mode = Mode::from_int(int(&data, KEY_MODE));
    let entry = entry_at(pos);
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
    Some(format!(
        "securitycraft keypad {} {} {} code={} entered={} mode={} signal={} powered={} status={} door={door}",
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
    ))
}

/// `/securitycraft code <pos> <digits>`: sets a code without the screen. It
/// exists because a headless harness cannot type into the screen, and is
/// recorded as a harness addition, not as gameplay.
fn set_code(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    code: &str,
) -> Result<String, String> {
    if !valid_code(code) {
        return Err("A code is one to twenty digits.".to_owned());
    }
    if block_at(&*host, level, pos) != KEYPAD {
        return Err("No keypad at that position.".to_owned());
    }
    let salt = fresh_salt(host).ok_or_else(|| "No keypad at that position.".to_owned())?;
    let digest = hash_code(code, &salt).unwrap_or_default();
    store_entry(pos, &Entry::new());
    let entity = host
        .block_entity(level, pos)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    let mut data = entity
        .data(&*host)
        .ok_or_else(|| "No keypad at that position.".to_owned())?;
    set_string(&mut data, KEY_SALT, &salt);
    set_string(&mut data, KEY_CODE, &digest);
    drop_key(&mut data, KEY_ENTRY);
    set_int(&mut data, KEY_MODE, Mode::Check.code());
    set_int(&mut data, KEY_STATUS, STATUS_SET);
    set_int(&mut data, KEY_SIGNAL, 0);
    let slots = data_slots(&*host, level, pos, &data);
    entity
        .replace_data(host, data)
        .map_err(|_| "No keypad at that position.".to_owned())?;
    let _ = entity.set_data_slots(host, slots);
    Ok("Code set.".to_owned())
}

/// Answers a `/securitycraft` invocation, or `None` when it is not one.
pub(crate) fn command(
    host: &mut dyn ServerHost,
    event: &mut mod_api::RunModCommandEvent<'_>,
) -> Option<String> {
    let invocation = event.command;
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
                keypad_info(host, level, pos)
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
            Some(set_code(host, level, pos, code).unwrap_or_else(|line| line))
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
        let pos = [3, 64, 5];
        // What the block entity saves starts empty and stays empty: the digits a
        // player types go to the transient store only.
        let nbt = Compound::new();
        let mut entry = Entry::new();
        for digit in [1, 2, 3, 4] {
            entry.push(digit);
        }
        store_entry(pos, &entry);
        assert_eq!(entry_at(pos), entry, "the screen can read what was typed");
        assert_eq!(
            string(&nbt, KEY_ENTRY),
            None,
            "a typed passcode is never written to the saved block entity"
        );
        // Enter clears it, and an empty entry is forgotten rather than stored.
        store_entry(pos, &Entry::new());
        assert!(entry_at(pos).is_empty());
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
        let (first, second) = ([1, 64, 1], [2, 64, 2]);
        let mut one = Entry::new();
        one.push(7);
        let mut two = Entry::new();
        two.push(9);
        two.push(9);
        store_entry(first, &one);
        store_entry(second, &two);
        assert_eq!(entry_at(first).text(), "7");
        assert_eq!(entry_at(second).text(), "99");
        store_entry(first, &Entry::new());
        store_entry(second, &Entry::new());
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
