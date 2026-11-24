#!/usr/bin/env bash
# STATUS (SDK migration, 2026-11-18): this is the retired `.hnmod` bundle's scenario, kept as the
# behaviour oracle. The port is now one flat SDK package (`securitycraft.hmod` at the port root,
# compiled into the game by the launcher), so the game this script drives must be a
# **launcher-built** one (`horizon-launcher native-build . --instance <instance>
# --trust-native-code`); `GAME=` points at it. The package carries the blocks, the code and the
# assets in one file, so the game is built against it and there is no `--dev-mod` leg. The
# `EMBEDDED=*` legs are retired with the embedded-content helper.
#
# In-game check for SecurityCraft slice 1: the keypad and the reinforced door.
#
# Upstream: a keypad stores a salted passcode; a player opens its screen,
# types the code and presses enter; a match cycles the keypad's POWERED state
# for `signalLength` ticks (60), and the reinforced door beside it opens on
# that redstone signal and closes when it ends. The port keeps the same shape:
# the code is a salted SHA-256 digest in the block entity, the screen is a drawn
# keypad whose keys arrive as menu buttons, and the door is the carried
# `minecraft:iron_door` template.
#
# Phases (Overworld, creative, a stone platform at y=100):
#  1. PLACE: the reinforced door item places both halves, the key panel places
#     the keypad; the door reads `open=false`.
#  2. SET: a fresh keypad opens in set mode; 1 2 3 4 + OK stores a code
#     (`code=set`, `mode=check`) and does not open anything.
#  3. WRONG: 9 9 9 9 + OK leaves the door closed (`status=1`, `powered=false`).
#  4. RIGHT: 1 2 3 4 + OK powers the keypad (`powered=true`) and the adjacent
#     reinforced door reads `open=true`.
#  5. CLOSE: after the signal length the keypad de-powers and the door closes.
#  6. UNLINKED (differential): a second keypad with the same code, no door
#     beside it, powers on the same right code, and the door stays closed.
#  7. NO CODE: a third, fresh keypad opens in set mode (`mode=set`) and OK with
#     nothing typed stores nothing (`code=none`, `status=4`).
#  8. WRONG BLOCK: an empty-hand click on the reinforced door opens no menu and
#     leaves it closed; so does a vanilla iron door (the control).
#  9. RECIPE: seven iron, paper and an arrow craft the harming module at a
#     crafting table; the grid is emptied (the ingredients are consumed).
# 10. OWNER + MODULE: the placer is the keypad's owner; the owner installs the
#     module; a wrong code takes two hearts and a right code none.
# 11. DROPS: breaking the keypad in survival with a pickaxe drops the module.
# 11b. LEVEL REMOVAL: `/setblock` (no loot hook, no player break) still leaves
#     the installed module in the world as a dropped item.
# 12. DIFFERENTIAL: a keypad without the module does not hurt a wrong code.
# 13. OWNER GATE: only the keypad's owner may set its first code. A single player cannot
#     become a stranger, so `/securitycraft owner <pos> stranger` (a harness command)
#     moves the owner: a stranger's click opens no set screen, a set screen that outlives
#     an owner change stores nothing, the owner's set works, and a stranger's
#     `/securitycraft code` is refused while the owner's code survives.
# 14. ALLOWLIST: craft the jar's allowlist module, install it on a coded keypad, set the
#     acting player with the harness command and click with nothing typed: the keypad
#     powers and the adjacent door opens without a code screen. Swapping the player for
#     a stranger makes the same click open a code screen and leave the door closed.
# 16. DENYLIST: craft the jar's denylist module (ink sac in the last row), install it on a
#     coded keypad with an adjacent door, and put the acting player on it: the click opens
#     nothing (`screen=playing`), the keypad stays unpowered and the door stays closed.
#     Swapping the list to stranger makes the identical click open the code screen, still
#     with no power and a closed door. `SELFTEST=deny` inverts the first assertion.
# 17. SMART: craft the jar's smart module (ender pearl in the last row) and install it on a
#     second coded keypad with a door. Five wrong codes followed by the right one must be
#     refused (`status=6`, unpowered, door closed) — upstream gates the cooldown on that
#     module. Phase 3b proves the same five wrong codes do NOT lock out a keypad without
#     any module. `SELFTEST=smart` inverts the phase 3b no-module assertion.
# 15. RELAUNCH: the same world is quit and started again; the code stored before the
#     restart still opens the same keypad and a wrong code is still refused, so the
#     digest is recomputed from the saved salt rather than held in the session.
#
#   tools/e2e.sh               # the scenario above
#   SELFTEST=1 tools/e2e.sh    # phase 4 expects the door CLOSED: must exit 1
#   SELFTEST=harming tools/e2e.sh  # phase 12 expects the missing damage: exit 1
#   SELFTEST=nonloot tools/e2e.sh  # phase 11b expects the module to have vanished: exit 1
#   SELFTEST=owner tools/e2e.sh    # phase 13b expects the stranger's code stored: exit 1
#   SELFTEST=relaunch tools/e2e.sh # phase 15 expects the stored digest lost: exit 1
#   SELFTEST=allow tools/e2e.sh    # phase 14 expects an allowed click not to power: exit 1
#   SELFTEST=deny tools/e2e.sh     # phase 16 expects a denied click to open a screen: exit 1
#   SELFTEST=smart tools/e2e.sh    # phase 3b expects a no-module cooldown: exit 1
#   CONTROL=1 tools/e2e.sh     # no package at all: the ids must be unknown
#   GAME=<an unmodified game> tools/e2e.sh   # the differential: the first phase fails
#
# Harness notes:
#  * the keypad screen has no visible slots, so the panel's origin comes from
#    `ctl ui`'s gui size: origin = ((w - 176) / 2, (h - 226) / 2);
#  * keys are hit-tested in `native_screen_click` and sent as menu buttons, so
#    `ctl click-at` at the key's centre is the whole input path;
#  * `/securitycraft info` is the harness readout (the task records it as such).
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/securitycraft
CTL=${CTL:-$PORT/target/horizon-cargo/fast/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/securitycraft-e2e.sock}
SELFTEST=${SELFTEST:-0}
# CONTROL=1: prove the mod's ids are unknown on a game without the package.
CONTROL=${CONTROL:-0}
LOG=${LOG:-$CLONE/ports/_shared/evidence/securitycraft/e2e.log}

mkdir -p "$(dirname "$LOG")"
exec > >(tee "$LOG") 2>&1

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/securitycraft.hmod" ] || { echo "missing package: $PORT/securitycraft.hmod" >&2; exit 1; }
command -v jq >/dev/null || { echo "missing jq" >&2; exit 1; }

KEYPAD=securitycraft:keypad
DOOR=securitycraft:iron_door_reinforced

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/securitycraft-home.XXXXXX)
# `--game-dir` makes the game `set_current_dir` into its game folder before anything runs
# (`game.rs`), so the embedded-content helper's `.horizon-cache` lands here, not in $SCRATCH.
GAME_HOME=$SCRATCH/game
export HORIZON_HOME=$GAME_HOME
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
say() { echo "== $* =="; }
block_name() { ctl block "$1" "$2" "$3" | jq -r '.block // "unknown"'; }
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // "unknown"'; }
door_open() { block_state "$1" "$2" "$3" | sed -n 's/.*open=\(true\|false\).*/\1/p'; }
# /securitycraft info prints one chat line; the reply is the parser's.
info() { ctl chat "/securitycraft info $1 $2 $3" >/dev/null; ctl wait 6 >/dev/null; ctl messages 8 | jq -r '.chat[]' | grep -m1 '^securitycraft keypad ' || true; }
field() { sed -n "s/.*[ ]$1=\([^ ]*\).*/\1/p" <<<"$2"; }
code_cmd() { ctl chat "/securitycraft code $1 $2 $3 $4" >/dev/null; ctl wait 4 >/dev/null; }
# A slot of the open menu by the item it holds (`ctl screen`'s own numbering).
inv_slot_of() { ctl screen | jq -r --arg i "$1" '[.slots[]? | select(.slot < 36 and .item == $i) | .slot] | first // ""'; }
# One item of ITEM from its inventory slot into the crafting grid CELL (a right
# click places one, the second left click puts the rest back).
grid_put_one() {
    local src
    src=$(inv_slot_of "$1")
    [ -n "$src" ] || fail "no $1 in the inventory for the recipe"
    ctl click-slot "$src" >/dev/null || fail "cannot pick up $1"
    ctl wait 2 >/dev/null
    ctl click-slot "$2" right >/dev/null || fail "cannot place $1 in grid cell $2"
    ctl wait 2 >/dev/null
    ctl click-slot "$src" >/dev/null || fail "cannot put the rest of $1 back"
    ctl wait 2 >/dev/null
}
health() { ctl state | jq -r '.player.health'; }

# The keypad panel's top-left in GUI pixels, from the open screen's size.
panel_origin() {
    local ui w h
    ui=$(ctl ui)
    w=$(jq -r '.gui_size[0] | floor' <<<"$ui")
    h=$(jq -r '.gui_size[1] | floor' <<<"$ui")
    OX=$(( (w - 176) / 2 ))
    OY=$(( (h - 226) / 2 ))
}

open_screen() { # open_screen X Y Z: stand south of the keypad, then click it
    ctl close >/dev/null 2>&1 || true
    ctl tp "$1.5" "$2" "$(( $3 + 2 )).5" >/dev/null
    ctl wait 6
    ctl command "clear" >/dev/null
    ctl wait 2
    ctl look 180 0
    ctl wait 2
    ctl look-at block "$1" "$2" "$3" south >/dev/null 2>&1 || true
    ctl interact "$1" "$2" "$3" south >/dev/null || fail "the keypad did not accept the right click"
    ctl wait 6
    [ "$(ctl screen | jq -r '.screen')" = "container" ] || fail "the keypad screen did not open: $(ctl screen | jq -r '.screen')"
    panel_origin
}

press() { # press COLUMN ROW (0-based key grid) -> clicks the key's centre
    local x=$(( OX + 52 + $1 * 26 + 10 )) y=$(( OY + 46 + $2 * 22 + 9 ))
    ctl click-at "$x" "$y" >/dev/null || fail "the key press at $x,$y did not send"
    ctl wait 2
}

press_digit() {
    local digit=$1 row col
    if [ "$digit" = 0 ]; then row=3; col=1; else row=$(( (digit - 1) / 3 )); col=$(( (digit - 1) % 3 )); fi
    press "$col" "$row"
}
press_backspace() { press 0 3; }
press_submit() { press 2 3; }
type_code() { local digit; for digit in $(echo "$1" | grep -o .); do press_digit "$digit"; done; }

# A right click on the keypad, reporting the screen it opened (or `playing` when the
# click was refused) without failing on a refusal.
click_keypad() { # X Y Z -> .screen
    ctl close >/dev/null 2>&1 || true
    ctl tp "$1.5" "$2" "$(( $3 + 2 )).5" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "clear" >/dev/null
    ctl wait 2 >/dev/null
    ctl look 180 0 >/dev/null
    ctl wait 2 >/dev/null
    ctl look-at block "$1" "$2" "$3" south >/dev/null 2>&1 || true
    ctl interact "$1" "$2" "$3" south >/dev/null 2>&1 || fail "the keypad did not accept the right click"
    ctl wait 6 >/dev/null
    ctl screen | jq -r '.screen'
}

# `/securitycraft owner` is the harness's way to put a stranger in front of a keypad:
# a single player cannot become a second one.
owner_cmd() { ctl chat "/securitycraft owner $1 $2 $3 $4" >/dev/null; ctl wait 4; }

# `/securitycraft allow` sets the allowlist module's one player. A single-player harness
# cannot become a second player, so `self` and `stranger` are the two sides of the check.
allow_cmd() { ctl chat "/securitycraft allow $1 $2 $3 $4" >/dev/null; ctl wait 4; }

# `/securitycraft deny` sets the denylist module's one player, the counterpart of allow_cmd.
deny_cmd() { ctl chat "/securitycraft deny $1 $2 $3 $4" >/dev/null; ctl wait 4; }

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    echo "EXIT:$status"
}
trap cleanup EXIT

# The package is compiled into the game the launcher built; no `--dev-mod`.
launch() { # launch WORLD [GAME]
    rm -f "$SOCK"
    local game=${2:-$GAME}
    echo "  launch: $(basename "$game"), no --dev-mod: the package carries its content" >&2
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "$1" --seed 20261105 --creative --offscreen --wait \
        --game-dir "$SCRATCH/game" --resources "$RES"
    ctl wait-ready 180
    if grep -q "content mods not loaded" "${SOCK}.log" 2>/dev/null; then
        grep "content mods not loaded" "${SOCK}.log" >&2
        fail "the content mod did not load"
    fi
}

# Fail-capability: with no content loaded the game must not know the mod's ids, so a PASS
# that resolved them cannot be vacuous.
ids_unknown() {
    local px pz reply block msgs
    say "control: the mod's item id is unknown"
    if reply=$(ctl give securitycraft:keypad_item 1 2>&1); then
        fail "the game gave an item from a package it did not load: $reply"
    fi
    echo "give: $reply"
    say "control: the module item id is unknown"
    if reply=$(ctl give securitycraft:harming_module 1 2>&1); then
        fail "the game gave the module from a package it did not load: $reply"
    fi
    echo "give module: $reply"
    px=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
    pz=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
    px=$((px + 2)); pz=$((pz + 2))
    say "control: the mod's block id is unknown to setblock"
    # A refused setblock still replies ok:true; the refusal is the chat line.
    reply=$(ctl command "setblock $px 100 $pz securitycraft:keypad" 2>&1) || true
    echo "setblock: $reply"
    grep -q "Unknown block type 'securitycraft:keypad'" <<<"$reply" \
        || fail "setblock accepted a block from a package that was not loaded: $reply"
    block=$(block_name "$px" 100 "$pz")
    echo "block at $px,100,$pz: $block"
    [ "$block" != securitycraft:keypad ] || fail "an unloaded package's keypad is in the world"
    say "control: /securitycraft does not exist"
    ctl chat "/securitycraft info $px 100 $pz" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    msgs=$(ctl messages 8 | jq -r '.chat[]' 2>/dev/null || true)
    if grep -q '^securitycraft keypad ' <<<"$msgs"; then
        fail "the harness command answered without the package"
    fi
    echo "chat: $(grep -i 'securitycraft' <<<"$msgs" | head -1 || echo 'no /securitycraft reply')"
}

say "launch (offscreen renderer, creative, the package compiled into the game)"
if [ "$CONTROL" = 1 ]; then
    say "control: no package is loaded"
    [ -x "${CONTROL_GAME:-}" ] || fail "CONTROL=1 needs CONTROL_GAME=<an unmodified client>"
    launch SecurityCraftE2E "$CONTROL_GAME"
    ids_unknown
    echo "PASS: without the package the game does not know securitycraft:keypad_item or securitycraft:keypad, setblock refuses the block, no keypad exists at the target and /securitycraft answers nothing"
    exit 0
fi
launch SecurityCraftE2E

say "an Overworld platform at y=100, away from the terrain"
PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
ctl command "fill $((PX-8)) 100 $((PZ-8)) $((PX+8)) 100 $((PZ+8)) minecraft:stone" >/dev/null
ctl command "fill $((PX-8)) 101 $((PZ-8)) $((PX+8)) 110 $((PZ+8)) minecraft:air" >/dev/null
ctl tp "$((PX+1)).5" 101 "$((PZ+3)).5" >/dev/null
ctl wait 20
ctl command "gamerule spawn_mobs false" >/dev/null
ctl command "gamerule spawn_monsters false" >/dev/null
ctl command "kill @e[type=!minecraft:player]" >/dev/null
ctl wait 20
ctl close >/dev/null 2>&1 || true
ctl look 180 0
ctl wait 6

DOOR_X=$PX; DOOR_Z=$PZ
KEY_X=$((PX+1)); KEY_Y=101; KEY_Z=$PZ

say "phase 1: the reinforced door item places both halves, the key panel places a keypad"
ctl give "$DOOR" 2 >/dev/null || fail "the game refused $DOOR: the package is not loaded"
ctl give securitycraft:keypad_item 3 >/dev/null || fail "the game refused securitycraft:keypad_item: the package is not loaded"
ctl wait 6
ctl select "$DOOR" >/dev/null
ctl place "$DOOR_X" 101 "$DOOR_Z" >/dev/null || fail "the reinforced door item did not place"
ctl wait 4
block_name "$DOOR_X" 101 "$DOOR_Z" | grep -qx "$DOOR" || fail "no door lower half at $DOOR_X,101,$DOOR_Z"
block_name "$DOOR_X" 102 "$DOOR_Z" | grep -qx "$DOOR" || fail "no door upper half at $DOOR_X,102,$DOOR_Z"
echo "door: $(block_state "$DOOR_X" 101 "$DOOR_Z")"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "the placed door is open"
ctl select securitycraft:keypad_item >/dev/null
ctl place "$KEY_X" "$KEY_Y" "$KEY_Z" >/dev/null || fail "the key panel did not place"
ctl wait 4
block_name "$KEY_X" "$KEY_Y" "$KEY_Z" | grep -qx "$KEYPAD" || fail "no keypad at $KEY_X,$KEY_Y,$KEY_Z"
echo "keypad: $(block_state "$KEY_X" "$KEY_Y" "$KEY_Z")"

say "phase 2: a fresh keypad opens in set mode; a three-digit code is refused, 1 2 3 4 + OK stores one"
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
type_code 123
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "after a three-digit set: $line"
[ "$(field status "$line")" = 5 ] || fail "a three-digit set was not refused: $line"
[ "$(field code "$line")" = none ] || fail "a three-digit code was stored: $line"
type_code 1234
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "after set: $line"
[ "$(field code "$line")" = set ] || fail "the code was not stored: $line"
[ "$(field mode "$line")" = check ] || fail "the screen did not switch to check mode: $line"
[ "$(field powered "$line")" = false ] || fail "setting a code powered the keypad: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "setting a code opened the door"
ctl close >/dev/null 2>&1 || true
ctl wait 4

say "phase 3: a wrong code leaves the door closed"
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
type_code 9999
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "after wrong code: $line"
[ "$(field status "$line")" = 1 ] || fail "the wrong code was not refused: $line"
[ "$(field powered "$line")" = false ] || fail "the wrong code powered the keypad: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "the wrong code opened the door"
ctl close >/dev/null 2>&1 || true
ctl wait 4
if [ "$SELFTEST" = 1 ]; then
    # Fail-capability: the same assertion must fail when it is wrong.
    [ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = true ] \
        || fail "SELFTEST: expected the door OPEN after a wrong code"
fi

say "phase 3b: a closed screen starts empty, and five wrong codes do not lock out a keypad without the smart module"
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
type_code 12
ctl close >/dev/null 2>&1 || true
ctl wait 4
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "after reopening: $line"
[ -z "$(field entered "$line")" ] || fail "digits survived the screen close: $line"
# Phase 3 spent one wrong code; four more would be five. Upstream only starts a
# keypad cooldown when the smart module is installed, so the right code still opens.
for _ in 1 2 3 4; do
    type_code 9999
    press_submit
done
type_code 1234
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "right code after five wrong codes, with no module: $line"
if [ "$SELFTEST" = smart ]; then
    # Fail-capability: invert the no-module assertion — a cooldown must NOT apply.
    [ "$(field status "$line")" = 6 ] \
        || fail "SELFTEST: expected a no-module keypad to be rate-limited"
fi
[ "$(field status "$line")" = 3 ] || fail "the right code was rate-limited without the smart module: $line"
[ "$(field powered "$line")" = true ] || fail "the right code did not power the keypad: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = true ] || fail "the right code did not open the door"
ctl close >/dev/null 2>&1 || true
# The no-module keypad is powered for signalLength ticks; let it close before phase 4.
ctl wait 80

say "phase 4: the right code powers the keypad and opens the adjacent reinforced door"
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
type_code 1234
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "after right code: $line"
[ "$(field powered "$line")" = true ] || fail "the right code did not power the keypad: $line"
[ "$(field signal "$line")" -gt 0 ] || fail "the signal length was not started: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = true ] || fail "the powered keypad did not open the door beside it"
ctl close >/dev/null 2>&1 || true

say "phase 5: the signal ends and the door closes"
ctl wait 70
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "after the signal: $line"
[ "$(field powered "$line")" = false ] || fail "the keypad stayed powered: $line"
[ "$(field signal "$line")" = 0 ] || fail "the signal did not run out: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "the door stayed open after the signal ended"

say "phase 6 (differential): a keypad with the same code but no door powers on and opens nothing"
UNLINKED_X=$((PX+4)); UNLINKED_Z=$PZ
ctl close >/dev/null 2>&1 || true
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
ctl place "$UNLINKED_X" "$KEY_Y" "$UNLINKED_Z" >/dev/null || fail "the second keypad did not place"
ctl wait 4
block_name "$UNLINKED_X" "$KEY_Y" "$UNLINKED_Z" | grep -qx "$KEYPAD" || fail "no second keypad"
code_cmd "$UNLINKED_X" "$KEY_Y" "$UNLINKED_Z" 1234
open_screen "$UNLINKED_X" "$KEY_Y" "$UNLINKED_Z"
type_code 1234
press_submit
line=$(info "$UNLINKED_X" "$KEY_Y" "$UNLINKED_Z")
echo "unlinked keypad: $line"
[ "$(field powered "$line")" = true ] || fail "the unlinked keypad did not power on: $line"
[ "$(field door "$line")" = none ] || fail "the unlinked keypad found a door: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "the unlinked keypad opened the first door"
ctl close >/dev/null 2>&1 || true
ctl wait 70

say "phase 7: a keypad with no code opens in set mode and OK with nothing typed stores nothing"
FRESH_X=$((PX+4)); FRESH_Z=$((PZ+4))
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
ctl place "$FRESH_X" "$KEY_Y" "$FRESH_Z" >/dev/null || fail "the third keypad did not place"
ctl wait 4
open_screen "$FRESH_X" "$KEY_Y" "$FRESH_Z"
line=$(info "$FRESH_X" "$KEY_Y" "$FRESH_Z")
echo "fresh keypad: $line"
[ "$(field code "$line")" = none ] || fail "the fresh keypad has a code: $line"
[ "$(field mode "$line")" = set ] || fail "the fresh keypad did not open in set mode: $line"
press_submit
line=$(info "$FRESH_X" "$KEY_Y" "$FRESH_Z")
echo "after an empty submit: $line"
[ "$(field status "$line")" = 4 ] || fail "an empty submit was not refused: $line"
[ "$(field code "$line")" = none ] || fail "an empty submit stored a code: $line"
ctl close >/dev/null 2>&1 || true
ctl wait 4

say "phase 8 (control): the reinforced door does not open by hand"
ctl close >/dev/null 2>&1 || true
ctl tp "$((DOOR_X+1)).5" 101 "$((DOOR_Z+2)).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl wait 2
ctl look-at block "$DOOR_X" 101 "$DOOR_Z" south >/dev/null 2>&1 || true
ctl interact "$DOOR_X" 101 "$DOOR_Z" south >/dev/null 2>&1 || true
ctl wait 4
[ "$(ctl screen | jq -r '.screen')" = "playing" ] || fail "a hand click on the reinforced door opened a screen"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "a hand click opened the reinforced door"
ctl close >/dev/null 2>&1 || true

say "phase 9: the harming module's recipe crafts it from seven iron, paper and an arrow"
ctl close >/dev/null 2>&1 || true
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 2
ctl give minecraft:crafting_table 1 >/dev/null
ctl give minecraft:iron_ingot 7 >/dev/null
ctl give minecraft:paper 1 >/dev/null
ctl give minecraft:arrow 1 >/dev/null
ctl wait 6
TABLE_X=$((PX-4)); TABLE_Z=$((PZ-4))
ctl tp "$(( TABLE_X + 1 )).5" 101 "$(( TABLE_Z + 2 )).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl select minecraft:crafting_table >/dev/null
ctl place "$TABLE_X" 101 "$TABLE_Z" >/dev/null || fail "the crafting table did not place"
ctl wait 4
block_name "$TABLE_X" 101 "$TABLE_Z" | grep -qx minecraft:crafting_table || fail "no crafting table at $TABLE_X,101,$TABLE_Z"
open_screen_table() {
    ctl close >/dev/null 2>&1 || true
    ctl tp "$(( $1 + 1 )).5" 101 "$(( $2 + 2 )).5" >/dev/null
    ctl wait 6
    ctl look 180 0
    ctl wait 2
    ctl look-at block "$1" 101 "$2" south >/dev/null 2>&1 || true
    ctl interact "$1" 101 "$2" south >/dev/null || fail "the crafting table did not take the right click"
    ctl wait 6
    # The crafting table reports `inventory` in this build; its grid is slots 100..108 and
    # the result 200 (the harness's slot numbering, as rftoolsstorage's e2e records).
    local screen grid
    screen=$(ctl screen | jq -r '.screen')
    grid=$(ctl screen | jq '[.slots[]? | select(.slot >= 100)] | length')
    [ "$grid" = 10 ] || fail "the crafting table grid did not open (screen=$screen, $grid grid slots)"
}
open_screen_table "$TABLE_X" "$TABLE_Z"
# The grid is 100..108 (row-major) and the result 200. Pattern: III / IPI / ISI.
for cell in 100 101 102 103 105 106 108; do grid_put_one minecraft:iron_ingot "$cell"; done
grid_put_one minecraft:paper 104
grid_put_one minecraft:arrow 107
ctl wait 4
result=$(ctl screen | jq -r '[.slots[]? | select(.slot == 200) | .item][0] // "empty"')
echo "recipe result: $result"
[ "$result" = securitycraft:harming_module ] || fail "the harming module recipe produced '$result'"
ctl click-slot 200 shift >/dev/null || fail "cannot take the crafted module"
ctl wait 4
left=$(ctl screen | jq -r '[.slots[]? | select(.slot >= 100 and .slot <= 108 and .item != null)] | length')
echo "grid slots still holding an item: $left"
[ "$left" = 0 ] || fail "the recipe did not consume its ingredients ($left grid slots filled)"
held=$(inv_slot_of securitycraft:harming_module)
echo "module in inventory slot: ${held:-none}"
[ -n "$held" ] || fail "the crafted module is not in the inventory"
ctl close >/dev/null 2>&1 || true
ctl wait 4

say "phase 10: the owner installs the module; a wrong code hurts, a right code does not"
MODULE_X=$((PX-4)); MODULE_Y=101; MODULE_Z=$((PZ+4))
ctl tp "$(( MODULE_X + 1 )).5" 101 "$(( MODULE_Z + 2 )).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
ctl place "$MODULE_X" "$MODULE_Y" "$MODULE_Z" >/dev/null || fail "the module keypad did not place"
ctl wait 4
line=$(info "$MODULE_X" "$MODULE_Y" "$MODULE_Z")
echo "fresh module keypad: $line"
[ "$(field owner "$line")" = set ] || fail "the placer did not become the keypad's owner: $line"
[ "$(field modules "$line")" = none ] || fail "a fresh keypad already has a module: $line"
code_cmd "$MODULE_X" "$MODULE_Y" "$MODULE_Z" 1234
ctl select securitycraft:harming_module >/dev/null
ctl tp "$(( MODULE_X + 1 )).5" 101 "$(( MODULE_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl look-at block "$MODULE_X" "$MODULE_Y" "$MODULE_Z" south >/dev/null 2>&1 || true
ctl interact "$MODULE_X" "$MODULE_Y" "$MODULE_Z" south >/dev/null || fail "the keypad did not take the module"
ctl wait 6
line=$(info "$MODULE_X" "$MODULE_Y" "$MODULE_Z")
echo "after installing the module: $line"
[ "$(field modules "$line")" = harming ] || fail "the harming module was not installed: $line"
[ "$(field code "$line")" = set ] || fail "installing the module lost the code: $line"
# Creative keeps the module (upstream's `!isCreative()` guard), so it is still carried.
carried=$(ctl inventory | jq -r '.counts["securitycraft:harming_module"] // 0')
echo "module still carried after a creative install: $carried"
[ "$carried" -ge 1 ] || fail "the creative install consumed the module"
# Survival so the damage is taken (a creative player is invulnerable to it).
ctl command "gamemode survival @s" >/dev/null
ctl wait 4
before=$(health)
open_screen "$MODULE_X" "$MODULE_Y" "$MODULE_Z"
type_code 9999
press_submit
# Read health before anything waits: full food regenerates a point every 10 ticks.
after=$(health)
line=$(info "$MODULE_X" "$MODULE_Y" "$MODULE_Z")
echo "wrong code with the harming module: $line health $before -> $after"
[ "$(field status "$line")" = 1 ] || fail "the wrong code was not refused: $line"
awk -v b="$before" -v a="$after" 'BEGIN { exit !(b - a >= 3.99) }' \
    || fail "the harming module did not take two hearts ($before -> $after)"
ctl close >/dev/null 2>&1 || true
ctl wait 4
before=$(health)
open_screen "$MODULE_X" "$MODULE_Y" "$MODULE_Z"
type_code 1234
press_submit
line=$(info "$MODULE_X" "$MODULE_Y" "$MODULE_Z")
after=$(health)
echo "right code with the harming module: $line health $before -> $after"
[ "$(field powered "$line")" = true ] || fail "the right code did not power the keypad: $line"
awk -v b="$before" -v a="$after" 'BEGIN { exit !(a >= b) }' \
    || fail "a right code was hurt ($before -> $after)"
ctl close >/dev/null 2>&1 || true
ctl command "gamemode creative @s" >/dev/null
ctl wait 4

say "phase 11: breaking the keypad drops the installed module"
# Survival with the right tool: a creative break drops nothing, and the keypad
# `requires_tool`, so a bare hand would drop nothing either.
ctl command "gamemode survival @s" >/dev/null
ctl give minecraft:diamond_pickaxe 1 >/dev/null
# `give` lands on the next tick; selecting before it arrives fails.
ctl wait 6
ctl select minecraft:diamond_pickaxe >/dev/null || fail "could not select the pickaxe for the break"
ctl wait 6
echo "breaking as $(ctl state | jq -r '.gamemode') with $(ctl inventory | jq -r '.held.item // "nothing"')"
broken=$(ctl break "$MODULE_X" "$MODULE_Y" "$MODULE_Z" --timeout 40) \
    || fail "the module keypad did not break: $broken"
echo "break: $(echo "$broken" | jq -c '{block, drops}')"
[ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "the module keypad did not break"
# The module may be on the ground or already picked up; either way it survived.
module_kept=$(echo "$broken" | jq -r '((.drops["securitycraft:harming_module"] // 0) + (.picked_up["securitycraft:harming_module"] // 0))')
[ "$module_kept" -ge 1 ] || fail "the installed harming module did not drop (or land in the inventory)"

say "phase 11b: a level-side removal leaves the installed module in the world"
# `/setblock` replaces the block through `Level.setBlock`: `Block.dropResources`
# never runs, so neither the loot table nor `block_drops` fires, and no player
# broke it. The module is the block entity's own content, so the block entity's
# removal must drop it — otherwise the block and its module both vanish.
NL_X=$((PX+4)); NL_Y=101; NL_Z=$((PZ-4))
ctl command "gamemode creative @s" >/dev/null
# No module entity from the phase 11 break may be counted here.
ctl command "kill @e[type=minecraft:item]" >/dev/null
ctl give securitycraft:keypad_item 1 >/dev/null
ctl give securitycraft:harming_module 1 >/dev/null
ctl wait 6
ctl tp "$(( NL_X + 1 )).5" 101 "$(( NL_Z + 2 )).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl select securitycraft:keypad_item >/dev/null
ctl place "$NL_X" "$NL_Y" "$NL_Z" >/dev/null || fail "the level-removal keypad did not place"
ctl wait 4
code_cmd "$NL_X" "$NL_Y" "$NL_Z" 1234
ctl select securitycraft:harming_module >/dev/null
ctl tp "$(( NL_X + 1 )).5" 101 "$(( NL_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl look-at block "$NL_X" "$NL_Y" "$NL_Z" south >/dev/null 2>&1 || true
ctl interact "$NL_X" "$NL_Y" "$NL_Z" south >/dev/null || fail "the keypad did not take the module"
ctl wait 6
line=$(info "$NL_X" "$NL_Y" "$NL_Z")
echo "keypad before the level-side removal: $line"
[ "$(field modules "$line")" = harming ] || fail "the module was not installed before the removal: $line"
# The level removes it: no loot roll, no player break.
ctl command "setblock $NL_X $NL_Y $NL_Z minecraft:air" >/dev/null
ctl wait 6
echo "after setblock: $(block_name "$NL_X" "$NL_Y" "$NL_Z")"
[ "$(block_name "$NL_X" "$NL_Y" "$NL_Z")" = minecraft:air ] || fail "setblock did not remove the keypad"
items=$(ctl entities 8 minecraft:item | jq -c '[.entities[] | {item: .item.item, count: .item.count, pos: .pos}]')
echo "items near the removed keypad: $items"
count=$(jq '[.[] | select(.item == "securitycraft:harming_module")] | length' <<<"$items")
if [ "$SELFTEST" = nonloot ]; then
    # Fail-capability: invert the assertion — the module must NOT survive.
    [ "$count" -eq 0 ] || fail "SELFTEST: expected the module to vanish with a level-side removal"
fi
[ "$count" -ge 1 ] || fail "the installed module vanished with a level-side removal (no item entity)"

say "phase 12 (differential): a keypad without the module does not hurt a wrong code"
DIFF_X=$((PX-4)); DIFF_Y=101; DIFF_Z=$PZ
ctl tp "$(( DIFF_X + 1 )).5" 101 "$(( DIFF_Z + 2 )).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
ctl place "$DIFF_X" "$DIFF_Y" "$DIFF_Z" >/dev/null || fail "the differential keypad did not place"
ctl wait 4
code_cmd "$DIFF_X" "$DIFF_Y" "$DIFF_Z" 1234
ctl command "gamemode survival @s" >/dev/null
ctl wait 4
before=$(health)
open_screen "$DIFF_X" "$DIFF_Y" "$DIFF_Z"
type_code 9999
press_submit
after=$(health)
line=$(info "$DIFF_X" "$DIFF_Y" "$DIFF_Z")
echo "wrong code without the module: $line health $before -> $after"
[ "$(field modules "$line")" = none ] || fail "the differential keypad has a module: $line"
[ "$(field status "$line")" = 1 ] || fail "the wrong code was not refused: $line"
awk -v b="$before" -v a="$after" 'BEGIN { exit !(a >= b) }' \
    || fail "a keypad without the harming module hurt the player ($before -> $after)"
if [ "$SELFTEST" = harming ]; then
    # Fail-capability: the differential expects the damage that must NOT happen.
    awk -v b="$before" -v a="$after" 'BEGIN { exit !(b - a >= 3.99) }' \
        || fail "SELFTEST: expected a keypad without the module to hurt the player"
fi
ctl close >/dev/null 2>&1 || true
ctl command "gamemode creative @s" >/dev/null
ctl wait 4

say "phase 13: only the owner may set a keypad's first code"
# A single-player harness cannot drive a second player, so `/securitycraft owner …
# stranger` is the counterfactual: it puts an owner who is not the acting player in
# front of the keypad (upstream's owner is the only gate on `verifyPasscodeSet`).
OWN_X=$((PX-4)); OWN_Y=101; OWN_Z=$((PZ+4))
ctl command "gamemode creative @s" >/dev/null
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 6
ctl tp "$(( OWN_X + 1 )).5" 101 "$(( OWN_Z + 2 )).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl select securitycraft:keypad_item >/dev/null
ctl place "$OWN_X" "$OWN_Y" "$OWN_Z" >/dev/null || fail "the owner keypad did not place"
ctl wait 4
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "owner keypad: $line"
[ "$(field owner "$line")" = set ] || fail "the placer did not become the owner: $line"
[ "$(field code "$line")" = none ] || fail "the owner keypad has a code: $line"
# (a) the set screen does not open for a stranger.
owner_cmd "$OWN_X" "$OWN_Y" "$OWN_Z" stranger
screen=$(click_keypad "$OWN_X" "$OWN_Y" "$OWN_Z")
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "a stranger's right click opened '$screen': $line"
[ "$screen" = playing ] || fail "a stranger opened the keypad's set screen ($screen)"
[ "$(field status "$line")" = 7 ] || fail "the stranger's click was not refused: $line"
[ "$(field code "$line")" = none ] || fail "a stranger's click stored a code: $line"
# (b) a set screen that outlives the ownership change stores nothing either.
owner_cmd "$OWN_X" "$OWN_Y" "$OWN_Z" self
open_screen "$OWN_X" "$OWN_Y" "$OWN_Z"
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "the owner's set screen: $line"
[ "$(field mode "$line")" = set ] || fail "the owner did not get the set screen: $line"
owner_cmd "$OWN_X" "$OWN_Y" "$OWN_Z" stranger
type_code 1234
press_submit
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "after a stranger's set submit: $line"
[ "$(field status "$line")" = 1 ] || fail "the stranger's set was not refused: $line"
[ "$(field code "$line")" = none ] || fail "a stranger stored the keypad's code: $line"
if [ "$SELFTEST" = owner ]; then
    # Fail-capability: invert the assertion — the stranger's code must be stored.
    [ "$(field code "$line")" = set ] || fail "SELFTEST: expected a stranger's code to be stored"
fi
ctl close >/dev/null 2>&1 || true
ctl wait 4
# (c) the owner sets it: the same digits now store the code.
owner_cmd "$OWN_X" "$OWN_Y" "$OWN_Z" self
open_screen "$OWN_X" "$OWN_Y" "$OWN_Z"
type_code 1234
press_submit
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "after the owner's set submit: $line"
[ "$(field code "$line")" = set ] || fail "the owner could not set the code: $line"
[ "$(field mode "$line")" = check ] || fail "the set did not switch to check mode: $line"
# (d) the harness control obeys the same rule: a stranger's `/securitycraft code` is refused,
# and the owner's code survives untouched.
ctl close >/dev/null 2>&1 || true
ctl wait 4
owner_cmd "$OWN_X" "$OWN_Y" "$OWN_Z" stranger
ctl chat "/securitycraft code $OWN_X $OWN_Y $OWN_Z 5678" >/dev/null
ctl wait 4
reply=$(ctl messages 4 | jq -r '.chat[]' | grep -m1 "Only the keypad's owner" || true)
echo "a stranger's code command answered: ${reply:-nothing}"
[ -n "$reply" ] || fail "a stranger's /securitycraft code was not refused with a reason"
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "after a stranger's code command: $line"
[ "$(field code "$line")" = set ] || fail "a stranger's /securitycraft code replaced the owner's code: $line"
open_screen "$OWN_X" "$OWN_Y" "$OWN_Z"
type_code 1234
press_submit
line=$(info "$OWN_X" "$OWN_Y" "$OWN_Z")
echo "the owner's code after the refused set: $line"
[ "$(field powered "$line")" = true ] || fail "the owner's code no longer opens the keypad: $line"
ctl close >/dev/null 2>&1 || true
owner_cmd "$OWN_X" "$OWN_Y" "$OWN_Z" self
ctl close >/dev/null 2>&1 || true
ctl command "gamemode creative @s" >/dev/null
ctl wait 4

say "phase 14: the allowlist module opens a coded keypad for a listed player without a code"
# Upstream's `KeypadBlock.useWithoutItem` checks the allowlist after `verifyPasscodeSet`:
# a listed player is activated (no code screen), while everyone else gets the code screen.
# The module is carried from the jar (whitelist_module: III/IPI/ISI, iron + two paper),
# the player list is set by the harness command (upstream's customize screen cannot be
# driven headlessly), and both the activation and its refusal are read back in game.
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 2
ctl give minecraft:crafting_table 1 >/dev/null
ctl give minecraft:iron_ingot 7 >/dev/null
ctl give minecraft:paper 2 >/dev/null
ctl wait 6
ATABLE_X=$((PX-6)); ATABLE_Z=$((PZ+6))
ctl tp "$(( ATABLE_X + 1 )).5" 101 "$(( ATABLE_Z + 2 )).5" >/dev/null
ctl wait 6
ctl look 180 0
ctl select minecraft:crafting_table >/dev/null
ctl place "$ATABLE_X" 101 "$ATABLE_Z" >/dev/null || fail "the allowlist crafting table did not place"
ctl wait 4
open_screen_table "$ATABLE_X" "$ATABLE_Z"
for cell in 100 101 102 103 105 106 108; do grid_put_one minecraft:iron_ingot "$cell"; done
grid_put_one minecraft:paper 104
grid_put_one minecraft:paper 107
ctl wait 4
result=$(ctl screen | jq -r '[.slots[]? | select(.slot == 200) | .item][0] // "empty"')
echo "allowlist recipe result: $result"
[ "$result" = securitycraft:allowlist_module ] || fail "the allowlist module recipe produced '$result'"
ctl click-slot 200 shift >/dev/null || fail "cannot take the crafted allowlist module"
ctl wait 4
left=$(ctl screen | jq -r '[.slots[]? | select(.slot >= 100 and .slot <= 108 and .item != null)] | length')
echo "allowlist grid slots still holding an item: $left"
[ "$left" = 0 ] || fail "the allowlist recipe did not consume its ingredients ($left grid slots filled)"
held=$(inv_slot_of securitycraft:allowlist_module)
echo "allowlist module in inventory slot: ${held:-none}"
[ -n "$held" ] || fail "the crafted allowlist module is not in the inventory"
ctl close >/dev/null 2>&1 || true
ctl wait 4

ALLOW_DOOR_X=$((PX-1)); ALLOW_DOOR_Z=$((PZ+4))
ALLOW_KEY_X=$((PX-2)); ALLOW_KEY_Y=101; ALLOW_KEY_Z=$((PZ+4))
ctl give "$DOOR" 1 >/dev/null
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 6
ctl tp "$(( ALLOW_DOOR_X + 1 )).5" 101 "$(( ALLOW_DOOR_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl select "$DOOR" >/dev/null
ctl place "$ALLOW_DOOR_X" 101 "$ALLOW_DOOR_Z" >/dev/null || fail "the allowlist door did not place"
ctl wait 4
block_name "$ALLOW_DOOR_X" 101 "$ALLOW_DOOR_Z" | grep -qx "$DOOR" || fail "no allowlist door at $ALLOW_DOOR_X,101,$ALLOW_DOOR_Z"
[ "$(door_open "$ALLOW_DOOR_X" 101 "$ALLOW_DOOR_Z")" = false ] || fail "the allowlist door started open"
ctl tp "$(( ALLOW_KEY_X + 1 )).5" 101 "$(( ALLOW_KEY_Z + 2 )).5" >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
ctl place "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" >/dev/null || fail "the allowlist keypad did not place"
ctl wait 4
block_name "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" | grep -qx "$KEYPAD" || fail "no allowlist keypad"
code_cmd "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" 1234
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "coded allowlist keypad: $line"
[ "$(field code "$line")" = set ] || fail "the allowlist keypad has no code: $line"
[ "$(field modules "$line")" = none ] || fail "the allowlist keypad already has a module: $line"

ctl select securitycraft:allowlist_module >/dev/null
ctl tp "$(( ALLOW_KEY_X + 1 )).5" 101 "$(( ALLOW_KEY_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl look-at block "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" south >/dev/null 2>&1 || true
ctl interact "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" south >/dev/null || fail "the keypad did not take the allowlist module"
ctl wait 6
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "after installing the allowlist module: $line"
[ "$(field modules "$line")" = allowlist ] || fail "the allowlist module was not installed: $line"
[ "$(field code "$line")" = set ] || fail "installing the allowlist module lost the code: $line"
[ "$(field allow "$line")" = 0 ] || fail "a fresh allowlist module already has players: $line"

allow_cmd "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" self
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "after adding the acting player: $line"
[ "$(field allow "$line")" = 1 ] || fail "the acting player was not added to the allowlist: $line"
# The click must activate directly: no code screen is opened and nothing is typed.
ctl close >/dev/null 2>&1 || true
ctl tp "$(( ALLOW_KEY_X + 1 )).5" 101 "$(( ALLOW_KEY_Z + 2 )).5" >/dev/null
ctl wait 6
ctl command "clear" >/dev/null
ctl wait 2
ctl look 180 0
ctl look-at block "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" south >/dev/null 2>&1 || true
ctl interact "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" south >/dev/null || fail "the keypad did not accept the allowed click"
ctl wait 6
screen=$(ctl screen | jq -r '.screen')
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "allowed click with nothing typed: screen=$screen $line"
[ "$screen" = playing ] || fail "the allowed click opened a code screen: $screen"
[ "$(field powered "$line")" = true ] || fail "the allowed click did not power the keypad: $line"
[ "$(field signal "$line")" -gt 0 ] || fail "the allowed click did not start a signal: $line"
[ "$(door_open "$ALLOW_DOOR_X" 101 "$ALLOW_DOOR_Z")" = true ] || fail "the allowed click did not open the door beside it"
if [ "$SELFTEST" = allow ]; then
    # Fail-capability: invert the branch — an allowed click must NOT power the keypad.
    [ "$(field powered "$line")" = false ] \
        || fail "SELFTEST: expected an allowed click not to power the keypad"
fi

# The differential: swap the acting player for a stranger. The same click now
# reaches the ordinary code screen and leaves the door closed.
ctl wait 70
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "after the allowlist signal: $line"
[ "$(field powered "$line")" = false ] || fail "the allowlist keypad stayed powered: $line"
[ "$(door_open "$ALLOW_DOOR_X" 101 "$ALLOW_DOOR_Z")" = false ] || fail "the allowlist door stayed open"
allow_cmd "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z" stranger
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "after adding a stranger: $line"
[ "$(field allow "$line")" = 1 ] || fail "the allowlist did not swap to the stranger: $line"
screen=$(click_keypad "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
line=$(info "$ALLOW_KEY_X" "$ALLOW_KEY_Y" "$ALLOW_KEY_Z")
echo "non-listed click: screen=$screen $line"
[ "$screen" = container ] || fail "a non-listed player did not get the code screen: $screen"
[ "$(field powered "$line")" = false ] || fail "a non-listed click powered the keypad: $line"
[ "$(door_open "$ALLOW_DOOR_X" 101 "$ALLOW_DOOR_Z")" = false ] || fail "a non-listed click opened the door"
ctl close >/dev/null 2>&1 || true

say "phase 16: the denylist module refuses a listed player before the allowlist branch is reached"
# Upstream's `KeypadBlock.useWithoutItem` checks `isDenied` before `isAllowed`: a denied
# click gets a message and no interaction at all, so no screen opens and nothing powers.
# The module is the jar's `blacklist_module` (III/IPI/ISI, iron + paper + ink sac), carried
# under `securitycraft:denylist_module` as in 0.2.6's allowlist rename.
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 2
ctl give minecraft:iron_ingot 7 >/dev/null
ctl give minecraft:paper 1 >/dev/null
ctl give minecraft:ink_sac 1 >/dev/null
ctl wait 6
# Reuse the allowlist phase's crafting table (it is still standing at ATABLE_X/Z).
open_screen_table "$ATABLE_X" "$ATABLE_Z"
for cell in 100 101 102 103 105 106 108; do grid_put_one minecraft:iron_ingot "$cell"; done
grid_put_one minecraft:paper 104
grid_put_one minecraft:ink_sac 107
ctl wait 4
result=$(ctl screen | jq -r '[.slots[]? | select(.slot == 200) | .item][0] // "empty"')
echo "denylist recipe result: $result"
[ "$result" = securitycraft:denylist_module ] || fail "the denylist module recipe produced '$result'"
ctl click-slot 200 shift >/dev/null || fail "cannot take the crafted denylist module"
ctl wait 4
left=$(ctl screen | jq -r '[.slots[]? | select(.slot >= 100 and .slot <= 108 and .item != null)] | length')
echo "denylist grid slots still holding an item: $left"
[ "$left" = 0 ] || fail "the denylist recipe did not consume its ingredients ($left grid slots filled)"
held=$(inv_slot_of securitycraft:denylist_module)
echo "denylist module in inventory slot: ${held:-none}"
[ -n "$held" ] || fail "the crafted denylist module is not in the inventory"
ctl close >/dev/null 2>&1 || true
ctl wait 4

DENY_DOOR_X=$((PX+2)); DENY_DOOR_Z=$((PZ+6))
DENY_KEY_X=$((PX+3)); DENY_KEY_Y=101; DENY_KEY_Z=$((PZ+6))
ctl give "$DOOR" 1 >/dev/null
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 6
ctl tp "$(( DENY_DOOR_X + 1 )).5" 101 "$(( DENY_DOOR_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl select "$DOOR" >/dev/null
ctl place "$DENY_DOOR_X" 101 "$DENY_DOOR_Z" >/dev/null || fail "the denylist door did not place"
ctl wait 4
block_name "$DENY_DOOR_X" 101 "$DENY_DOOR_Z" | grep -qx "$DOOR" || fail "no denylist door"
[ "$(door_open "$DENY_DOOR_X" 101 "$DENY_DOOR_Z")" = false ] || fail "the denylist door started open"
ctl tp "$(( DENY_KEY_X + 1 )).5" 101 "$(( DENY_KEY_Z + 2 )).5" >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
ctl place "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" >/dev/null || fail "the denylist keypad did not place"
ctl wait 4
block_name "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" | grep -qx "$KEYPAD" || fail "no denylist keypad"
code_cmd "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" 1234
line=$(info "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
echo "coded denylist keypad: $line"
[ "$(field code "$line")" = set ] || fail "the denylist keypad has no code: $line"
[ "$(field modules "$line")" = none ] || fail "the denylist keypad already has a module: $line"

ctl select securitycraft:denylist_module >/dev/null
ctl tp "$(( DENY_KEY_X + 1 )).5" 101 "$(( DENY_KEY_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl look-at block "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" south >/dev/null 2>&1 || true
ctl interact "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" south >/dev/null || fail "the keypad did not take the denylist module"
ctl wait 6
line=$(info "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
echo "after installing the denylist module: $line"
[ "$(field modules "$line")" = denylist ] || fail "the denylist module was not installed: $line"
[ "$(field code "$line")" = set ] || fail "installing the denylist module lost the code: $line"
[ "$(field deny "$line")" = 0 ] || fail "a fresh denylist module already has players: $line"

deny_cmd "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" self
line=$(info "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
echo "after adding the acting player to the denylist: $line"
[ "$(field deny "$line")" = 1 ] || fail "the acting player was not added to the denylist: $line"
screen=$(click_keypad "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
line=$(info "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
echo "denied click: screen=$screen $line"
if [ "$SELFTEST" = deny ]; then
    # Fail-capability: invert the "a denied click opens nothing" assertion.
    [ "$screen" = container ] || fail "SELFTEST: expected the denied click to open a screen"
fi
[ "$screen" = playing ] || fail "a denied click opened a screen: $screen"
[ "$(field status "$line")" = 8 ] || fail "the denied click did not set the deny status: $line"
[ "$(field powered "$line")" = false ] || fail "a denied click powered the keypad: $line"
[ "$(door_open "$DENY_DOOR_X" 101 "$DENY_DOOR_Z")" = false ] || fail "a denied click opened the door"

# Counterfactual: swap the list to a stranger. The identical click now reaches the
# ordinary code screen, still with no power and a closed door.
deny_cmd "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z" stranger
line=$(info "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
echo "after swapping the denylist to a stranger: $line"
[ "$(field deny "$line")" = 1 ] || fail "the denylist did not swap to the stranger: $line"
screen=$(click_keypad "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
line=$(info "$DENY_KEY_X" "$DENY_KEY_Y" "$DENY_KEY_Z")
echo "non-denied click: screen=$screen $line"
[ "$screen" = container ] || fail "a non-denied player did not get the code screen: $screen"
[ "$(field powered "$line")" = false ] || fail "a non-denied click powered the keypad: $line"
[ "$(door_open "$DENY_DOOR_X" 101 "$DENY_DOOR_Z")" = false ] || fail "a non-denied click opened the door"
ctl close >/dev/null 2>&1 || true

say "phase 17: the smart module applies the retry cooldown that phase 3b showed is absent without it"
# Upstream's `IPasscodeProtected.onIncorrectPasscodeEntered` calls `startCooldown()`
# only when `isModuleEnabled(ModuleType.SMART)`; `CheckPasscode.handle` refuses a
# verification while that cooldown holds. The jar's smart_module recipe is
# III/IPI/ISI with an ender pearl (the pack's `c:ender_pearls` resolves to it).
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 2
ctl give minecraft:iron_ingot 7 >/dev/null
ctl give minecraft:paper 1 >/dev/null
ctl give minecraft:ender_pearl 1 >/dev/null
ctl wait 6
open_screen_table "$ATABLE_X" "$ATABLE_Z"
for cell in 100 101 102 103 105 106 108; do grid_put_one minecraft:iron_ingot "$cell"; done
grid_put_one minecraft:paper 104
grid_put_one minecraft:ender_pearl 107
ctl wait 4
result=$(ctl screen | jq -r '[.slots[]? | select(.slot == 200) | .item][0] // "empty"')
echo "smart recipe result: $result"
[ "$result" = securitycraft:smart_module ] || fail "the smart module recipe produced '$result'"
ctl click-slot 200 shift >/dev/null || fail "cannot take the crafted smart module"
ctl wait 4
left=$(ctl screen | jq -r '[.slots[]? | select(.slot >= 100 and .slot <= 108 and .item != null)] | length')
echo "smart grid slots still holding an item: $left"
[ "$left" = 0 ] || fail "the smart recipe did not consume its ingredients ($left grid slots filled)"
held=$(inv_slot_of securitycraft:smart_module)
echo "smart module in inventory slot: ${held:-none}"
[ -n "$held" ] || fail "the crafted smart module is not in the inventory"
ctl close >/dev/null 2>&1 || true
ctl wait 4

SMART_DOOR_X=$((PX-1)); SMART_DOOR_Z=$((PZ-2))
SMART_KEY_X=$((PX-2)); SMART_KEY_Y=101; SMART_KEY_Z=$((PZ-2))
ctl give "$DOOR" 1 >/dev/null
ctl give securitycraft:keypad_item 1 >/dev/null
ctl wait 6
ctl tp "$(( SMART_DOOR_X + 1 )).5" 101 "$(( SMART_DOOR_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl select "$DOOR" >/dev/null
ctl place "$SMART_DOOR_X" 101 "$SMART_DOOR_Z" >/dev/null || fail "the smart keypad's door did not place"
ctl wait 4
block_name "$SMART_DOOR_X" 101 "$SMART_DOOR_Z" | grep -qx "$DOOR" || fail "no smart door"
[ "$(door_open "$SMART_DOOR_X" 101 "$SMART_DOOR_Z")" = false ] || fail "the smart door started open"
ctl tp "$(( SMART_KEY_X + 1 )).5" 101 "$(( SMART_KEY_Z + 2 )).5" >/dev/null
ctl wait 4
ctl select securitycraft:keypad_item >/dev/null
echo "smart placement: target=$(block_name "$SMART_KEY_X" 101 "$SMART_KEY_Z") counts=$(ctl inventory | jq -c '.counts')"
reply=$(ctl place "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z" 2>&1) || { echo "smart place reply: $reply"; fail "the smart keypad did not place"; }
echo "smart place reply: $reply"
ctl wait 4
block_name "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z" | grep -qx "$KEYPAD" || fail "no smart keypad"
code_cmd "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z" 1234
line=$(info "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z")
echo "coded smart keypad: $line"
[ "$(field code "$line")" = set ] || fail "the smart keypad has no code: $line"
[ "$(field modules "$line")" = none ] || fail "the smart keypad already has a module: $line"

ctl select securitycraft:smart_module >/dev/null
ctl tp "$(( SMART_KEY_X + 1 )).5" 101 "$(( SMART_KEY_Z + 2 )).5" >/dev/null
ctl wait 4
ctl look 180 0
ctl look-at block "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z" south >/dev/null 2>&1 || true
ctl interact "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z" south >/dev/null || fail "the keypad did not take the smart module"
ctl wait 6
line=$(info "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z")
echo "after installing the smart module: $line"
[ "$(field modules "$line")" = smart ] || fail "the smart module was not installed: $line"
[ "$(field code "$line")" = set ] || fail "installing the smart module lost the code: $line"

open_screen "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z"
for _ in 1 2 3 4 5; do
    type_code 9999
    press_submit
done
type_code 1234
press_submit
line=$(info "$SMART_KEY_X" "$SMART_KEY_Y" "$SMART_KEY_Z")
echo "right code with the smart module: $line"
[ "$(field status "$line")" = 6 ] || fail "the smart module did not rate-limit the right code: $line"
[ "$(field powered "$line")" = false ] || fail "a locked-out attempt powered the smart keypad: $line"
[ "$(door_open "$SMART_DOOR_X" 101 "$SMART_DOOR_Z")" = false ] || fail "a locked-out attempt opened the smart door"
ctl close >/dev/null 2>&1 || true

say "phase 15: a code set before a restart still opens the same keypad after it"
# The digest is salted SHA-256 over data the keypad saved, so a restart re-reads the
# same salt and the same digest from the block entity and must recompute a match. A
# digest that only worked in the session that stored it would pass every phase above
# and fail here.
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "keypad before the relaunch: $line"
[ "$(field code "$line")" = set ] || fail "the keypad lost its code before the relaunch: $line"
ctl close >/dev/null 2>&1 || true
ctl quit >/dev/null 2>&1 || true
sleep 5
launch SecurityCraftE2E
ctl wait 20 >/dev/null
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "keypad after the relaunch: $line"
if [ "$SELFTEST" = relaunch ]; then
    # Fail-capability: invert the assertion — the restart must lose the stored digest.
    [ "$(field code "$line")" = none ] || fail "SELFTEST: expected the relaunch to lose the stored digest"
fi
[ "$(field code "$line")" = set ] || fail "the relaunch lost the stored digest: $line"
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
type_code 9999
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "a wrong code after the relaunch: $line"
[ "$(field status "$line")" = 1 ] || fail "a wrong code after the relaunch was not refused: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = false ] || fail "a wrong code after the relaunch opened the door"
open_screen "$KEY_X" "$KEY_Y" "$KEY_Z"
type_code 1234
press_submit
line=$(info "$KEY_X" "$KEY_Y" "$KEY_Z")
echo "the right code after the relaunch: $line"
[ "$(field powered "$line")" = true ] || fail "the code set before the relaunch no longer opens the keypad: $line"
[ "$(door_open "$DOOR_X" 101 "$DOOR_Z")" = true ] || fail "the keypad's signal after the relaunch did not open the door"
ctl close >/dev/null 2>&1 || true

echo "PASS: the key panel placed a keypad and the reinforced door item placed both halves; a fresh keypad set a code without opening anything (a three-digit code refused); a wrong code left the reinforced door closed; a reopened screen started empty and five wrong codes did NOT rate-limit a correct one without the smart module; the right code then powered the keypad and opened the door beside it; the door closed when the signal ran out; a second keypad with the same code but no door powered on and opened nothing; an empty submit stored nothing; and a hand click cannot open the reinforced door; the harming module recipe crafted from seven iron, paper and an arrow and consumed them; the placer became the keypad's owner, the owner installed the module, a wrong code took two hearts and a right code none, and the module dropped when the keypad broke — and survived a level-side /setblock removal as a dropped item; a keypad without the module did not hurt a wrong code; and only the owner may set a keypad's first code — a stranger's click opened no set screen, a set screen that outlived an owner change stored nothing, the owner set it, and a stranger's /securitycraft code was refused; the allowlist module crafted from seven iron and two paper was installed on a coded keypad and an allowed click with nothing typed powered it and opened its door without a code screen, while swapping the player for a stranger restored the code screen and left the door closed; the denylist module crafted from seven iron, paper and an ink sac was installed on a coded keypad and a listed player's click opened nothing, powered nothing and left the door closed, while swapping the list to a stranger restored the code screen with the door still closed; the smart module crafted from seven iron, paper and an ender pearl was installed on a second coded keypad and five wrong codes then made the right one read status=6 with the keypad unpowered and the door closed, while the same five wrong codes on a keypad without the module still opened it; and after quitting and relaunching the same world the code stored before the restart still opens the keypad and a wrong code is still refused"
# The cleanup trap prints the run's exit status as the last line.
