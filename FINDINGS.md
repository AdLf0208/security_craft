# SecurityCraft — upstream analysis (slice 1: keypad + reinforced door)

Ported from **SecurityCraft 1.10.2.1** (`[1.21.1] SecurityCraft v1.10.2.1.jar`,
SHA-1 `8760079e80039f982f1a92d3ee9d60a2bcab3251`, SHA-256
`75ac9e73c60caf58df7069f167dbacc00a640e1418207dff654f56a5fdb5f229`,
5,193,371 B), **MIT** (the jar's `META-INF/neoforge.mods.toml`) → **rank 0**.

Upstream read: `Geforce132/SecurityCraft`, branch `1.21.1`, commit
`c4d35b0c8bf85c2a94c37a1f8ce37ef5ce1a6aa8` (shallow clone
`ports/_shared/upstream/securitycraft`, deleted after this file was written).
Provenance and the licence evidence: `ports/_shared/carry-work/securitycraft/
SOURCE.md`. The source was read for behaviour only; this is a clean-room Rust
rewrite.

## Files read (behaviour, not code)

| File | What was taken from it |
|---|---|
| `blocks/KeypadBlock.java` | `POWERED`/`FACING`/`ROTATION`/`WATERLOGGED` states, `useWithoutItem` (powered → nothing; disabled; code-not-set; denylist/allowlist; else open the code screen), `activate` (cycle `POWERED`, update indirect neighbours, `scheduleTick(signalLength)`), `tick` (clear `POWERED`), `getSignal`/`getDirectSignal` (15 when powered), `isSignalSource` (always), placement facing (`getNearestLookingDirection().getOpposite()`), `onRemove` (drop the salt) |
| `blockentities/KeypadBlockEntity.java` | saved `passcode` + `saltKey`, the options (`signalLength` 60, `disabled`, smart-module cooldown, message toggles), `activate(player)`, `onOwnerChanged` (clears the code and the salt), `shouldAttemptCodebreak` |
| `api/IPasscodeProtected.java` | `openPasscodeGUI` (only when a code exists), `verifyPasscodeSet` (no code + owner → set screen; no code + stranger → "not set up"), `hashAndSetPasscode` (new random salt, PBKDF2), `savePasscodeAndSalt`/`loadSaltKey`/`loadPasscode` (32-char hash string, legacy ≤ 20-char plaintext is upgraded), `onIncorrectPasscodeEntered` |
| `network/server/SetPasscode.java` | set requires owner and not spectator; the code is hashed client-side (SHA-512 hex) and then salted/PBKDF2 server-side |
| `network/server/CheckPasscode.java` | per-player 250 ms cooldown, keypad cooldown (smart module), compare the salted digest, `activate(player)` on a match, `onIncorrectPasscodeEntered` otherwise |
| `screen/CheckPasscodeScreen.java` | a 176×186 panel, digits 1–9/0, backspace, check, a censoring text field, Enter/Esc, a cooldown lock-out |
| `screen/SetPasscodeScreen.java` | a 176×166 panel, a digit-only field, "save and continue", Enter |
| `util/PasscodeUtils.java` | random 16-byte salt, SHA-512 hex for the unsalted first hash, PBKDF2-HMAC-SHA1 (65,536 rounds, 128-bit) with the salt, hex strings in NBT |
| `blocks/reinforced/ReinforcedDoorBlock.java` | a vanilla-door shape/state block with owner data: opens when an adjacent "active" SecurityCraft block (a keypad that is `POWERED`) or a 15-signal neighbour powers it; a double door opens together; drop only from the lower half; iron-door sounds; `hasActiveSCBlockNextTo` also accepts a plain 15 signal through a non-source neighbour |
| `util/BlockUtils.java` (`hasActiveSCBlockNextTo`, `updateIndirectNeighbors`, `getDestroyProgress`) | the door activator check, the neighbour update the keypad does, and the owner-only breaking speed (the block's `destroySpeed` is `-1` except for its owner) |
| `SCContent.java` (the two registrations) | block ids `keypad` and `iron_door_reinforced`, item ids `keypad_item` (the key panel) and `door_indestructible_iron_item`, hardness 5.0, metal sounds, `requiresCorrectToolForDrops`, explosion resistance `Float.MAX_VALUE`; the reinforced door drops its own double-high item from the lower half |

## Behaviour extracted

- **Code storage:** a random 16-byte salt per keypad; the stored value is a
  one-way digest of the typed digits, never the digits (the code is not shown
  in NBT or on the item).
- **Setting:** a keypad without a code opens the *set* screen for its owner; a
  keypad with a code opens the *check* screen for everyone else (the owner too,
  unless an allowlist module opens it for them).
- **Checking:** the entered digits are compared to the digest; a match calls
  `activate`, which toggles `POWERED` and schedules the off tick
  (`signalLength`, default 60 ticks); a wrong code messages the player and does
  nothing.
- **The link:** there is no stored link. A powered keypad gives a 15 signal on
  every face; the reinforced door beside it opens on that signal and closes
  when it ends. That is what "the keypad is linked to the door" means upstream.
- **Facing:** the keypad faces the player who places it.
- **Powered keypad:** a right click while it is powered does nothing.

## Edge cases worth carrying

- Enter with nothing typed is a failed check, not a no-op (the digest of the
  empty string never matches).
- An unset keypad refuses a *check* even with the right digits: it must be set
  first.
- The off tick is scheduled for `signalLength` ticks, so a chunk reload during
  the signal keeps it (the countdown rides the block entity here).
- The door drops once, from the lower half, with the item that places both
  halves.
- A keypad's code disappears with the block entity (breaking it loses the
  code); a door cannot be opened by hand (iron-door behaviour).

## Out of scope for slice 1 (gaps, recorded in `../securitycraft/notes.md`)

Owner and allowlist/denylist messages; the smart-module cooldown; the harming
module and code-breaker damage; the disguise module and its tint; the
`disabled` option; waterlogging; wall/floor/ceiling mounting (4-way facing
only, see notes); keypad chests/barrels/furnaces/smokers/trapdoors/doors; the
universal block remover/reinforcer; cameras, mines, lasers, scanners, retinal
scanner, block pocket, alarm, display cases; the owner-only breaking speed and
the whole "reinforced block" family; the `frame` (disguisable frame) and its
`IPasscodeConvertible`; the keypad-door item; the reinforced door's own recipe
(upstream makes it by reinforcing an iron door with the Block Reinforcer).
