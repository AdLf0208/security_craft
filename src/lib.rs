//! SecurityCraft for Horizon: the keypad, the reinforced door and the harming module.
//!
//! The rules — the code's shape, the salted digest, the verdict of a submitted entry, the
//! screen mode, the key-to-button map, the signal countdown, who owns the keypad (and so who
//! may set its code or install a module), the allowlist's and denylist's listed players and the
//! damage a wrong code deals — live in
//! [`runtime`], which knows nothing about the game and carries its own tests. [`support`] is
//! the game glue, and this file is the one `impl Hooks`: the keypad's
//! [Parts](mod_api::BlockEntityPart), the key panel item's part, the module drop, and the
//! `/securitycraft` harness commands.
//!
//! A keypad is a mod block entity with a menu; a reinforced door beside it is carried content
//! (a vanilla iron door) that opens on the keypad's redstone signal and closes when the signal
//! runs out. The player who places the keypad owns it, and the owner may install the harming
//! module, which hurts a player who enters a wrong code.
pub mod runtime;
mod support;

use mod_api::{
    self, BlockChangedEvent, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent,
    BlockEntityTicksEvent, BlockPart, Flow, Hooks, HostSide, InteractionResult,
    IsSignalSourceEvent, ItemPart, LevelHost, LevelSide, LevelTickPostEvent, MenuButtonEvent,
    MenuClosedEvent, OnBlockPlacedByEvent, PlayerContainerCloseEvent, RegisterCommandsEvent,
    Registrar, RegistrationContext, RunModCommandEvent, Screen, ScreenClickEvent, ScreenDrawEvent,
    ServerHost, StateForPlacementEvent, UseItemOnEvent, UseWithoutItemEvent,
};

/// The keypad: its block entity (screen, menu, signal) and its block (placement, redstone).
struct Keypad;

#[mod_api::horizon_part]
impl BlockPart for Keypad {
    /// `BlockBehaviour.useItemOn`: a powered keypad does nothing, an unpowered one starts the
    /// check or set screen and lets the content block's own use open the menu.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_use(host, event.click)
    }

    /// `BlockBehaviour.useWithoutItem`: the same, with an empty hand.
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_use(host, event.click)
    }

    /// `BlockState.isSignalSource`: every keypad state is one.
    fn is_signal_source(&self, level: &dyn LevelHost, event: &mut IsSignalSourceEvent) {
        support::on_is_signal_source(level, event);
    }

    /// `BlockState.getSignal`: 15 on every face while powered.
    fn block_signal(&self, level: &dyn LevelHost, event: &mut mod_api::BlockSignalEvent) {
        support::on_block_signal(level, event);
    }

    /// `BlockBehaviour.getStateForPlacement`: the keypad faces the player who placed it.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<mod_api::BlockState>> {
        support::on_state_for_placement(level, event)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Keypad {
    /// `EntityBlock.newBlockEntity`: a fresh keypad saves nothing.
    fn block_entity(
        &self,
        _level: LevelSide<'_>,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::on_block_entity()
    }

    /// `BlockEntity.getUpdateTag`: send only the installed module ids to clients, so the
    /// disguise render can read them without the salted passcode digest leaving the server.
    fn block_entity_update_tag(
        &self,
        event: &mut mod_api::BlockEntityUpdateTagEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::update_tag(event)
    }

    /// `BlockEntityRenderer.submit`: draw the disguised keypad's tinted block on the client.
    fn block_entity_submit(
        &self,
        host: &dyn mod_api::ClientHost,
        event: &mut mod_api::BlockEntitySubmitEvent<'_>,
    ) {
        support::submit(host, event);
    }

    /// `EntityBlock.getTicker`: the keypad's signal counts down on the server.
    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        support::on_ticks(event);
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        support::on_tick(host, event);
    }

    /// The panel's layout: one hidden container slot and the player's inventory below it.
    fn screen(
        &self,
        _host: &dyn mod_api::Host,
        _event: &mut mod_api::ScreenEvent<'_>,
    ) -> Option<Screen> {
        Some(support::screen())
    }

    /// The code display and the twelve keys.
    fn screen_draw(&self, host: &dyn mod_api::ClientHost, event: &mut ScreenDrawEvent<'_>) {
        support::on_screen_draw(host, event);
    }

    /// A press on a drawn key names the menu button the server applies.
    fn screen_click(
        &self,
        _host: &dyn mod_api::ClientHost,
        event: &mut ScreenClickEvent<'_>,
    ) -> Option<i32> {
        support::on_screen_click(event)
    }

    /// The server applies that press to the block entity.
    fn menu_button(&self, host: &mut dyn ServerHost, event: &mut MenuButtonEvent<'_>) {
        support::on_menu_button(host, event);
    }
}

/// The key panel item: placing it makes the player who placed it the keypad's owner.
struct KeyPanel;

#[mod_api::horizon_part]
impl ItemPart for KeyPanel {
    /// `BlockItemExtension.onBlockPlacedBy`: the placer becomes the keypad's owner, and the
    /// owner is who may install modules on it (`on_use`) and open a fresh keypad's set screen.
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        support::on_block_placed_by(host, event);
    }
}

/// The passcode barrel: a block with a saved code and a one-shot open gate in front of its
/// 27-slot container. Its block item records the placer as the owner, as the key panel does.
struct ContainerItem;

#[mod_api::horizon_part]
impl ItemPart for ContainerItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        support::on_block_placed_by(host, event);
    }
}

/// The passcode barrel's block. It gates the click that would open the container menu.
struct ContainerBlock;

#[mod_api::horizon_part]
impl BlockPart for ContainerBlock {
    /// A click with an item: no code-verified open permission means the container's own use
    /// (which would open its menu) must not run.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_container_use(host, event.click)
    }

    /// The same, with an empty hand.
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_container_use(host, event.click)
    }
}

/// The passcode furnace's block item: placing it records the placer as the owner, as the
/// barrel's item does.
struct FurnaceItem;

#[mod_api::horizon_part]
impl ItemPart for FurnaceItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        support::on_block_placed_by(host, event);
    }
}

/// The passcode furnace's block: the same one-shot open gate as the barrel, in front of
/// the container's menu.
struct FurnaceBlock;

#[mod_api::horizon_part]
impl BlockPart for FurnaceBlock {
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_furnace_use(host, event.click)
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_furnace_use(host, event.click)
    }
}

/// The passcode furnace's block entity: saved owner/code, four furnace timing ints and the
/// three input/fuel/output slots the game backs.
struct FurnaceEntity;

#[mod_api::horizon_part]
impl BlockPart for FurnaceEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for FurnaceEntity {
    fn block_entity(
        &self,
        _level: LevelSide<'_>,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::on_block_entity()
    }

    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        support::on_ticks(event);
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        support::on_furnace_tick(host, event);
    }

    fn screen(
        &self,
        _host: &dyn mod_api::Host,
        _event: &mut mod_api::ScreenEvent<'_>,
    ) -> Option<Screen> {
        Some(support::furnace_screen())
    }
}

/// The passcode smoker's block item: placing it records the placer as the owner.
struct SmokerItem;

#[mod_api::horizon_part]
impl ItemPart for SmokerItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        support::on_block_placed_by(host, event);
    }
}

/// The passcode smoker's block: the same one-shot open gate as the furnace.
struct SmokerBlock;

#[mod_api::horizon_part]
impl BlockPart for SmokerBlock {
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_smoker_use(host, event.click)
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_smoker_use(host, event.click)
    }
}

/// The passcode smoker's block entity: saved owner/code, the four cook timing ints and the
/// three input/fuel/output slots, ticked against the game's `minecraft:smoking` recipes.
struct SmokerEntity;

#[mod_api::horizon_part]
impl BlockPart for SmokerEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for SmokerEntity {
    fn block_entity(
        &self,
        _level: LevelSide<'_>,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::on_block_entity()
    }

    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        support::on_ticks(event);
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        support::on_smoker_tick(host, event);
    }

    fn screen(
        &self,
        _host: &dyn mod_api::Host,
        _event: &mut mod_api::ScreenEvent<'_>,
    ) -> Option<Screen> {
        Some(support::smoker_screen())
    }
}

/// The passcode barrel's block entity: the code, the salt and the open gate, with the game
/// backing its 27 item slots. A separate type from the block Part — one Part type per block.
struct ContainerEntity;

#[mod_api::horizon_part]
impl BlockPart for ContainerEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for ContainerEntity {
    fn block_entity(
        &self,
        _level: LevelSide<'_>,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::on_block_entity()
    }

    /// A barrel's open/close animation and viewer count are ticked by its template; keep the
    /// block entity's ticker so carried containers behave as vanilla barrels do.
    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        support::on_ticks(event);
    }

    /// The 27-slot chest panel shown when the content menu opens.
    fn screen(
        &self,
        _host: &dyn mod_api::Host,
        _event: &mut mod_api::ScreenEvent<'_>,
    ) -> Option<Screen> {
        Some(support::barrel_screen())
    }
}

/// The passcode trapdoor's block item: placing it records the placer as owner,
/// as the barrel, furnace and smoker items do.
struct TrapdoorItem;

#[mod_api::horizon_part]
impl ItemPart for TrapdoorItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        support::on_block_placed_by(host, event);
    }
}

/// The passcode trapdoor's block: an already-open trapdoor answers `PASS` to a
/// right click (upstream's `useWithoutItem`); a hand click never opens an iron
/// trapdoor, so the carried template handles the rest.
struct TrapdoorBlock;

#[mod_api::horizon_part]
impl BlockPart for TrapdoorBlock {
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_trapdoor_use(host, event.click)
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        support::on_trapdoor_use(host, event.click)
    }
}

/// The passcode trapdoor's block entity: saved owner/code and the countdown
/// that closes the block's `open` state after `signalLength` (default 60).
struct TrapdoorEntity;

#[mod_api::horizon_part]
impl BlockPart for TrapdoorEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for TrapdoorEntity {
    fn block_entity(
        &self,
        _level: LevelSide<'_>,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::on_block_entity()
    }

    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        support::on_ticks(event);
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        support::on_trapdoor_tick(host, event);
    }
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the keypad's block entity, its part, and its one-slot container with the mod
    /// menu the block's use opens.
    fn register(&self, r: &mut dyn Registrar, cx: &RegistrationContext) {
        let block = r.block_key(support::KEYPAD);
        let kind = r
            .block_entity_type("keypad")
            .blocks(&[support::KEYPAD])
            .build();
        // Only our own duplicate declaration can refuse here, and without the part the keypad
        // would stand inert: refuse the mod's registration rather than load a half-working
        // keypad, so the game log names it.
        if let Err(error) = r.block_entity_part(&kind, Keypad) {
            cx.refuse(&format!(
                "securitycraft: the keypad part was refused: {error}"
            ));
            return;
        }
        r.block_container(&block, support::keypad_container());
        // Bound first: `r.item_part(r.item_key(…), …)` borrows `r` twice.
        let item = r.item_key(support::KEYPAD_ITEM);
        r.item_part(item, KeyPanel);
        // The passcode barrel: its own block entity, a 27-slot container shown as the
        // vanilla 9×3 menu, and the item whose placement records its owner.
        let container = r.block_key(support::CONTAINER);
        let container_kind = r
            .block_entity_type(support::CONTAINER_ENTITY)
            .blocks(&[support::CONTAINER])
            .build();
        if let Err(error) = r.block_entity_part(&container_kind, ContainerEntity) {
            cx.refuse(&format!(
                "securitycraft: the passcode barrel part was refused: {error}"
            ));
            return;
        }
        r.block_part(container, ContainerBlock);
        r.block_container(&container, support::barrel_container());
        let container_item = r.item_key(support::CONTAINER);
        r.item_part(container_item, ContainerItem);
        // The passcode furnace: its own block entity and a three-slot input/fuel/output
        // container that cooks against the game's smelting table on every tick.
        let furnace = r.block_key(support::FURNACE);
        let furnace_kind = r
            .block_entity_type(support::FURNACE_ENTITY)
            .blocks(&[support::FURNACE])
            .build();
        if let Err(error) = r.block_entity_part(&furnace_kind, FurnaceEntity) {
            cx.refuse(&format!(
                "securitycraft: the passcode furnace part was refused: {error}"
            ));
            return;
        }
        r.block_part(furnace, FurnaceBlock);
        r.block_container(&furnace, support::furnace_container());
        let furnace_item = r.item_key(support::FURNACE);
        r.item_part(furnace_item, FurnaceItem);
        // The passcode smoker: its own block entity and three-slot container, cooked against
        // the game's smoking table on every tick (the furnace's sibling in the jar).
        let smoker = r.block_key(support::SMOKER);
        let smoker_kind = r
            .block_entity_type(support::SMOKER_ENTITY)
            .blocks(&[support::SMOKER])
            .build();
        if let Err(error) = r.block_entity_part(&smoker_kind, SmokerEntity) {
            cx.refuse(&format!(
                "securitycraft: the passcode smoker part was refused: {error}"
            ));
            return;
        }
        r.block_part(smoker, SmokerBlock);
        r.block_container(&smoker, support::smoker_container());
        let smoker_item = r.item_key(support::SMOKER);
        r.item_part(smoker_item, SmokerItem);
        // The passcode trapdoor: its own block entity closes the carried iron
        // trapdoor's `open` state after the signal, and its item records the owner.
        let trapdoor = r.block_key(support::TRAPDOOR);
        let trapdoor_kind = r
            .block_entity_type(support::TRAPDOOR_ENTITY)
            .blocks(&[support::TRAPDOOR])
            .build();
        if let Err(error) = r.block_entity_part(&trapdoor_kind, TrapdoorEntity) {
            cx.refuse(&format!(
                "securitycraft: the passcode trapdoor part was refused: {error}"
            ));
            return;
        }
        r.block_part(trapdoor, TrapdoorBlock);
        let trapdoor_item = r.item_key(support::TRAPDOOR);
        r.item_part(trapdoor_item, TrapdoorItem);
        // A removal hook only has a `LevelHost`, which cannot resolve an id, so the
        // keys of all four droppable modules are stored once here.
        let harming_module = r.item_key(support::HARMING_MODULE);
        let allowlist_module = r.item_key(support::ALLOWLIST_MODULE);
        let denylist_module = r.item_key(support::DENYLIST_MODULE);
        let smart_module = r.item_key(support::SMART_MODULE);
        let disguise_module = r.item_key(support::DISGUISE_MODULE);
        support::set_module_items(
            harming_module,
            allowlist_module,
            denylist_module,
            smart_module,
            disguise_module,
        );
    }

    /// `/securitycraft info|code|owner`: the harness controls this slice's e2e needs (a headless
    /// harness cannot type into the screen, and a single player cannot become a stranger).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(mod_api::Command::new("securitycraft info <pos:block_pos>"));
        event.commands.push(mod_api::Command::op(
            "securitycraft code <pos:block_pos> <code:word>",
            2,
        ));
        event.commands.push(mod_api::Command::op(
            "securitycraft owner <pos:block_pos> <who:word>",
            2,
        ));
        event.commands.push(mod_api::Command::op(
            "securitycraft allow <pos:block_pos> <who:word>",
            2,
        ));
        event.commands.push(mod_api::Command::op(
            "securitycraft deny <pos:block_pos> <who:word>",
            2,
        ));
        // The passcode barrel's harness controls: a headless harness cannot type into a
        // screen, and no second player can be driven, so the code check and its owner are
        // exercised through these commands (recorded as harness additions in notes.md).
        event.commands.push(mod_api::Command::op(
            "securitycraft cset <pos:block_pos> <code:word>",
            2,
        ));
        event.commands.push(mod_api::Command::op(
            "securitycraft copen <pos:block_pos> <code:word>",
            2,
        ));
        event
            .commands
            .push(mod_api::Command::new("securitycraft cinfo <pos:block_pos>"));
    }

    /// A menu a mod opened closed, or its player left: every digit that player typed is gone.
    fn menu_closed(&self, _host: &mut dyn ServerHost, event: &mut MenuClosedEvent) {
        support::on_menu_closed(event.uuid);
    }

    /// The player's block menu closed (closed, out of reach, died, changed level or left): the
    /// keypad session at that block is forgotten as well.
    fn player_container_close(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerContainerCloseEvent<'_>,
    ) {
        if event.menu == support::KEYPAD_MENU {
            support::on_container_closed(event.player, event.dimension, event.pos);
        } else if event.menu == support::CONTAINER_MENU
            && let Some(pos) = event.pos
        {
            support::on_barrel_container_closed(host, event.player, event.dimension, pos);
        } else if event.menu == support::FURNACE_MENU
            && let Some(pos) = event.pos
        {
            support::on_furnace_container_closed(host, event.player, event.dimension, pos);
        } else if event.menu == support::SMOKER_MENU
            && let Some(pos) = event.pos
        {
            support::on_smoker_container_closed(host, event.player, event.dimension, pos);
        }
    }

    /// A state was committed. A keypad that is gone — a player's break,
    /// `/setblock`, `/fill`, a piston, an explosion — queues its installed
    /// modules for a drop; `block_drops` (a loot roll) is not asked for a
    /// level-side removal, so a module dropped only there vanishes with the
    /// block. Upstream's `IModuleInventory.dropAllModules`.
    fn block_changed(&self, level: &dyn LevelHost, event: &mut BlockChangedEvent) {
        support::on_block_changed(level, event);
    }

    /// The level ticked: the modules of a keypad removed during it are spawned
    /// as item entities (`block_changed` has a read-only level).
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        if let HostSide::Server(host) = host {
            support::on_level_tick_post(host, event);
        }
    }

    /// Runs a `/securitycraft` usage and reports its one line.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let Some(line) = support::command(host, event) else {
            return Flow::Continue;
        };
        support::feedback(host, &line);
        Flow::Cancel
    }
}
