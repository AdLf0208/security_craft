//! SecurityCraft for Horizon: the keypad and the reinforced door (slice 1).
//!
//! The rules — the code's shape, the salted digest, the verdict of a submitted entry, the
//! screen mode, the key-to-button map and the signal countdown — live in [`runtime`], which
//! knows nothing about the game and carries its own tests, copied byte for byte from the
//! retired `.hnmod` bundle. [`support`] is the game glue, and this file is the one
//! `impl Hooks`: the keypad's [Parts](mod_api::BlockEntityPart), its registration, and the
//! `/securitycraft` harness commands.
//!
//! A keypad is a mod block entity with a menu; a reinforced door beside it is carried content
//! (a vanilla iron door) that opens on the keypad's redstone signal and closes when the signal
//! runs out.
pub mod runtime;
mod support;

use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent,
    BlockPart, Flow, Hooks, InteractionResult, IsSignalSourceEvent, LevelHost, LevelSide,
    MenuButtonEvent, RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent,
    Screen, ScreenClickEvent, ScreenDrawEvent, ServerHost, StateForPlacementEvent, UseItemOnEvent,
    UseWithoutItemEvent,
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
    }

    /// `/securitycraft info|code`: the harness controls this slice's e2e needs (a headless
    /// harness cannot type into the screen).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(mod_api::Command::new("securitycraft info <pos:block_pos>"));
        event.commands.push(mod_api::Command::op(
            "securitycraft code <pos:block_pos> <code:word>",
            2,
        ));
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
