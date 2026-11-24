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
        r.block_container(&block, support::container());
        // Bound first: `r.item_part(r.item_key(…), …)` borrows `r` twice.
        let item = r.item_key(support::KEYPAD_ITEM);
        r.item_part(item, KeyPanel);
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
    }

    /// A menu a mod opened closed, or its player left: every digit that player typed is gone.
    fn menu_closed(&self, _host: &mut dyn ServerHost, event: &mut MenuClosedEvent) {
        support::on_menu_closed(event.uuid);
    }

    /// The player's block menu closed (closed, out of reach, died, changed level or left): the
    /// keypad session at that block is forgotten as well.
    fn player_container_close(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut PlayerContainerCloseEvent<'_>,
    ) {
        if event.menu == support::KEYPAD_MENU {
            support::on_container_closed(event.player, event.dimension, event.pos);
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
