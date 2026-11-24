// The keypad's access-control rules, pure and unit-tested.
//
// Upstream (`KeypadBlock`, `KeypadBlockEntity`, `IPasscodeProtected`,
// `CheckPasscode`, `SetPasscode`, `PasscodeUtils`) keeps a salted hash of the
// typed digits, verifies a submitted entry against it, and on a match cycles
// the keypad's `POWERED` state for `signalLength` ticks, which is what opens a
// reinforced door beside it through ordinary redstone. This module holds the
// parts of that which need no game: the entry, the digest, the verdict and the
// signal countdown. The glue in `support.rs` reads and writes the world.
//
// Upstream double-hashes (SHA-512 on the client, PBKDF2-HMAC-SHA1 with 65,536
// rounds and a random 16-byte salt on the server). The port keeps a salted
// SHA-256 digest: the same shape (a random salt per keypad, a one-way digest,
// the plain digits never saved) without a key-derivation loop on the server
// thread. Recorded as a deliberate deviation.
//
// The digest is computed by [`sha256`] below, this port's own implementation:
// the registry refuses a package that declares any dependency but the SDK
// (`tools/native-subset/src/manifests.rs`, R9), so a hash crate cannot travel
// with a native `.hmod`.

/// The shortest code a keypad accepts. A one-digit PIN is a 10-value secret: five
/// guesses reach 50%, so the server refuses anything shorter than four digits (a
/// review finding of 2026-11-18). Upstream's text box filters to digits but has no
/// minimum; this port's minimum is enforced on both the set screen and the
/// `/securitycraft code` control.
pub const MIN_CODE_LEN: usize = 4;

/// The longest entry the keypad accepts. Upstream's text box has no limit, but
/// its legacy passcodes were at most 20 characters; beyond that typos are the
/// only thing a longer code buys.
pub const MAX_CODE_LEN: usize = 20;

/// Failed check attempts allowed before a lockout. Five guesses against a
/// four-digit code leave a 0.05% chance; the lockout then costs time, not guesses.
pub const MAX_FAILED_ATTEMPTS: u32 = 5;

/// How long a keypad refuses verification after [`MAX_FAILED_ATTEMPTS`] failures, in
/// milliseconds. A per-player, per-keypad lockout (ten seconds) slows guessing without
/// locking other players out of the same keypad.
pub const LOCKOUT_MS: u64 = 10_000;

/// The default time the keypad stays powered after a correct code
/// (`SignalLengthOption(60)`, 60 ticks = 3 s).
pub const DEFAULT_SIGNAL_TICKS: i32 = 60;

/// Menu-button ids the keypad screen sends. 0..=9 are the digits, so the two
/// other keys start above them.
pub const BUTTON_BACKSPACE: i32 = 10;
pub const BUTTON_SUBMIT: i32 = 11;

/// A key on the keypad screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Digit(u8),
    Backspace,
    Submit,
}

/// The damage a wrong passcode deals when the keypad's harming module is installed:
/// `ConfigHandler.SERVER.incorrectPasscodeDamage`, two hearts = 4 health points.
pub const INCORRECT_PASSCODE_DAMAGE: f32 = 4.0;

/// Whether `player` may install a module on a keypad owned by `owner`.
///
/// Upstream's `ModuleItem.useOn` returns `PASS` unless `IOwnable.isOwnedBy(player)`:
/// the owner is the player who placed the block, and an unowned block (an old save,
/// a dispenser placement) installs nothing.
#[must_use]
pub fn may_install(owner: Option<u128>, player: u128) -> bool {
    owner == Some(player)
}

/// Whether `player` may set a keypad's code, first or again.
///
/// Upstream gates every set on ownership twice: `IPasscodeProtected.verifyPasscodeSet` opens
/// the set screen only for the block's owner (a stranger is told the block is "not set up"),
/// and `SetPasscode.handle` refuses the hashed code unless `IOwnable.isOwnedBy(player)` — so a
/// client-sent set from a screen that outlived the check stores nothing either. An unowned
/// keypad (a dispenser placement, an old save) is nobody's to set.
#[must_use]
pub fn may_set_code(owner: Option<u128>, player: u128) -> bool {
    owner == Some(player)
}

/// The damage a wrong submission deals, `None` without the harming module.
///
/// Upstream's `IPasscodeProtected.onIncorrectPasscodeEntered`: with the harming module
/// installed a wrong code hurts the player; a keypad without it only reports the failure.
#[must_use]
pub fn wrong_code_damage(harming: bool) -> Option<f32> {
    harming.then_some(INCORRECT_PASSCODE_DAMAGE)
}

/// The id the jar ships for its disguise module (`data/securitycraft/recipe/disguise_module.json`,
/// `assets/securitycraft/models/item/disguise_module.json`). The artefact's id wins over any name
/// the brief guessed from the mod's marketing.
pub const DISGUISE_MODULE: &str = "securitycraft:disguise_module";

/// The fixed tint (0xRRGGBB) a disguised keypad renders with in this port. Upstream's disguise
/// module carries a GUI-chosen block state (`DisguiseModuleScreen`, `SetStateOnDisguiseModule`);
/// the pin stamps no module screen, so the port fixes one distinctive appearance instead. The
/// simplification is recorded as a deviation in `notes.md`.
pub const DISGUISE_TINT: u32 = 0x00_FF_00;

/// The tint a keypad carrying `modules` renders with, or `None` for its own look.
///
/// Upstream's `IModuleInventory.isModuleEnabled(ModuleType.DISGUISE)` gates the disguised render;
/// the check is by id so it is order-independent and survives other modules being installed.
#[must_use]
pub fn disguise_tint(modules: &[String]) -> Option<u32> {
    modules
        .iter()
        .any(|module| module == DISGUISE_MODULE)
        .then_some(DISGUISE_TINT)
}

/// The most players a list module carries (upstream `ListModuleData.MAX_PLAYERS`).
pub const MAX_LIST_PLAYERS: usize = 50;

/// The customizable data of a list module: the players on it and whether it affects
/// everyone. Upstream's `ListModuleData` also carries a team list; this slice has no
/// team surface in the SDK, so that field is not carried (recorded as a deviation).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListData {
    players: Vec<String>,
    affect_everyone: bool,
}

impl ListData {
    /// An empty list, as a freshly crafted module carries.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a player, up to [`MAX_LIST_PLAYERS`], without duplicating a name.
    #[must_use]
    pub fn with_player(mut self, player: &str) -> Self {
        if self.players.len() >= MAX_LIST_PLAYERS || self.contains(player) {
            return self;
        }
        self.players.push(player.to_owned());
        self
    }

    /// Sets whether everyone is affected, as upstream's `updateAffectEveryone`.
    #[must_use]
    pub fn affecting_everyone(mut self, affects: bool) -> Self {
        self.affect_everyone = affects;
        self
    }

    /// The players on the list, in insertion order.
    #[must_use]
    pub fn players(&self) -> &[String] {
        &self.players
    }

    /// Whether this list affects everyone rather than just the named players.
    #[must_use]
    pub fn affects_everyone(&self) -> bool {
        self.affect_everyone
    }

    /// Whether `player` is on the list, compared case-insensitively as upstream's
    /// `ListModuleData.isPlayerOnList` does.
    #[must_use]
    pub fn contains(&self, player: &str) -> bool {
        self.players
            .iter()
            .any(|listed| listed.eq_ignore_ascii_case(player))
    }
}

/// Whether an allowlist module opens the keypad for `player` without a code.
///
/// Upstream's `IModuleInventory.isAllowed`: a keypad without the module allows
/// nobody; with one, a listed player (or everyone, when the list is set that way)
/// is allowed and `KeypadBlock.useWithoutItem` calls `activate` without a code.
#[must_use]
pub fn is_allowed(installed: bool, data: &ListData, player: &str) -> bool {
    installed && (data.affects_everyone() || data.contains(player))
}

/// Whether a denylist module refuses `player` any interaction with the keypad.
///
/// Upstream's `IModuleInventory.isDenied`: a keypad without the module denies
/// nobody; with one, `affectEveryone` denies everyone except the owner, otherwise
/// the team/player lists deny. `KeypadBlock.useWithoutItem` checks deny **before**
/// allow, so a listed player on both lists is refused. The owner exception is the
/// glue's (the pure rule receives only the acting player's key).
#[must_use]
pub fn is_denied(installed: bool, data: &ListData, player: &str) -> bool {
    installed && (data.affects_everyone() || data.contains(player))
}

/// The key a menu-button id stands for, `None` for an id the screen never
/// sends.
#[must_use]
pub fn button(id: i32) -> Option<Button> {
    match id {
        0..=9 => Some(Button::Digit(u8::try_from(id).ok()?)),
        BUTTON_BACKSPACE => Some(Button::Backspace),
        BUTTON_SUBMIT => Some(Button::Submit),
        _ => None,
    }
}

/// Whether `code` is something a player could set: [`MIN_CODE_LEN`] to
/// [`MAX_CODE_LEN`] ASCII digits. Upstream's text box filters to `\d*`, so letters
/// never reach the server; the minimum is this port's (see [`MIN_CODE_LEN`]).
#[must_use]
pub fn valid_code(code: &str) -> bool {
    (MIN_CODE_LEN..=MAX_CODE_LEN).contains(&code.len())
        && code.bytes().all(|byte| byte.is_ascii_digit())
}

/// The hex digest a keypad stores: SHA-256 over the salt's bytes followed by
/// the code's digits. `None` when the code or the salt is malformed, so a
/// corrupt save cannot become a code that matches everything.
#[must_use]
pub fn hash_code(code: &str, salt_hex: &str) -> Option<String> {
    if !valid_code(code) {
        return None;
    }
    let mut message = decode_hex(salt_hex)?;
    message.extend_from_slice(code.as_bytes());
    Some(encode_hex(&sha256(&message)))
}

/// A salt as the hex a keypad saves, from the bytes the level's random gave.
#[must_use]
pub fn new_salt(bytes: &[u8]) -> String {
    encode_hex(bytes)
}

/// What a submitted entry means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No code has been set on this keypad yet (`getPasscode() == null`).
    NotSet,
    /// Nothing was typed; upstream's check screen sends it and the server's
    /// comparison fails.
    Empty,
    /// The digest matches the stored one.
    Correct,
    /// Anything else.
    Incorrect,
}

/// Compares the typed `entry` with the keypad's stored digest.
#[must_use]
pub fn submit(entry: &str, salt_hex: &str, stored: Option<&str>) -> Verdict {
    let Some(stored) = stored else {
        return Verdict::NotSet;
    };
    if entry.is_empty() {
        return Verdict::Empty;
    }
    match hash_code(entry, salt_hex) {
        Some(digest) if digest == stored => Verdict::Correct,
        _ => Verdict::Incorrect,
    }
}

/// The digits typed into an open keypad screen since it opened.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    text: String,
}

impl Entry {
    /// An empty entry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The digits typed so far.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many digits were typed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether no digit was typed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Adds one digit, refusing a tenth-plus digit and a non-digit.
    pub fn push(&mut self, digit: u8) -> bool {
        if digit > 9 || self.len() >= MAX_CODE_LEN {
            return false;
        }
        self.text.push(char::from(b'0' + digit));
        true
    }

    /// Removes the last digit; false when there was none.
    pub fn backspace(&mut self) -> bool {
        self.text.pop().is_some()
    }

    /// Forgets every digit, as opening the screen or submitting does.
    pub fn clear(&mut self) {
        self.text.clear();
    }

    /// The code display: one `*` per typed digit, never the digits themselves
    /// (upstream's censoring text box unless "show passcode" is ticked).
    #[must_use]
    pub fn masked(&self) -> String {
        "*".repeat(self.len())
    }
}

/// The direction the keypad's face points when a player with `yaw` places it:
/// the opposite of the way they look, as upstream's
/// `getNearestLookingDirection().getOpposite()` does. Vanilla's quarter turns
/// are `floor(yaw / 90 + 0.5) mod 4` = south, west, north, east.
#[must_use]
pub fn facing_from_yaw(yaw: f32) -> &'static str {
    let quarter = ((yaw / 90.0 + 0.5).floor() as i64).rem_euclid(4);
    ["north", "east", "south", "west"][usize::try_from(quarter).unwrap_or_default()]
}

/// Whether the open keypad screen checks an entered code or sets the first one
/// (`openPasscodeGUI` vs `openSetPasscodeScreen`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Ask for the code (a code is set).
    Check,
    /// Ask for a code to store (the keypad has none).
    Set,
}

impl Mode {
    /// The mode a saved `mode` field names; anything but `1` is check.
    #[must_use]
    pub fn from_int(value: i32) -> Self {
        if value == 1 { Self::Set } else { Self::Check }
    }

    /// The mode as the block entity saves it.
    #[must_use]
    pub fn code(self) -> i32 {
        match self {
            Self::Check => 0,
            Self::Set => 1,
        }
    }
}

/// What the keypad does with a submitted entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// The code matched: power the keypad.
    Open,
    /// The code did not match.
    Wrong,
    /// A valid new code was typed in set mode; store its digest.
    Set,
    /// Enter was pressed with nothing typed.
    Empty,
    /// Enter was pressed with a code shorter than [`MIN_CODE_LEN`] (or longer than
    /// [`MAX_CODE_LEN`]): refused before any comparison.
    TooShort,
    /// A check on a keypad with no code becomes a set.
    Check,
}

/// Decides what Enter means, from the screen's mode, the typed entry, the
/// keypad's salt and its stored digest. Upstream: the set screen saves any
/// non-empty digit string, and the check screen compares the digest.
#[must_use]
pub fn after_submit(mode: Mode, entry: &str, salt_hex: &str, stored: Option<&str>) -> Action {
    if entry.is_empty() {
        return Action::Empty;
    }
    if !valid_code(entry) {
        return Action::TooShort;
    }
    match mode {
        Mode::Set => Action::Set,
        Mode::Check => match submit(entry, salt_hex, stored) {
            Verdict::Correct => Action::Open,
            Verdict::Incorrect | Verdict::Empty => Action::Wrong,
            Verdict::NotSet => Action::Check,
        },
    }
}

/// A per-keypad, per-player verification budget: [`MAX_FAILED_ATTEMPTS`] wrong
/// codes, then a [`LOCKOUT_MS`] pause. Kept beside the session rather than in the
/// block entity, so it is not saved and cannot be reset by reopening the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Throttle {
    failures: u32,
    locked_until_ms: u64,
}

impl Throttle {
    /// Whether this session may verify a code at `now_ms` (a monotonic clock in
    /// milliseconds; the glue passes the system clock).
    #[must_use]
    pub fn allows(&self, now_ms: u64) -> bool {
        now_ms >= self.locked_until_ms
    }

    /// Record a wrong code. The fifth failure starts a lockout; a later failure after
    /// the lockout starts another.
    pub fn failed(&mut self, now_ms: u64) {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= MAX_FAILED_ATTEMPTS {
            self.failures = 0;
            self.locked_until_ms = now_ms.saturating_add(LOCKOUT_MS);
        }
    }

    /// Record a correct code: the budget resets.
    pub fn succeeded(&mut self) {
        self.failures = 0;
        self.locked_until_ms = 0;
    }
}

/// Whether a verification may go ahead on a keypad that may carry the smart module.
///
/// Upstream's keypad-side cooldown exists only with that module: `IPasscodeProtected`
/// `onIncorrectPasscodeEntered` calls `startCooldown()` under
/// `isModuleEnabled(ModuleType.SMART)`, and `CheckPasscode.handle` refuses a
/// verification while `isOnCooldown()` holds. Without the module only the separate
/// per-player 250 ms spam guard remains (not modelled in this port). The retry
/// budget below is this port's form of that cooldown.
#[must_use]
pub fn cooldown_allows(smart: bool, throttle: &Throttle, now_ms: u64) -> bool {
    !smart || throttle.allows(now_ms)
}

/// Records a wrong code against the smart-module cooldown budget.
///
/// Without the module upstream records nothing on the keypad, so no later install
/// can inherit a lockout earned before it was there.
pub fn cooldown_failed(smart: bool, throttle: &mut Throttle, now_ms: u64) {
    if smart {
        throttle.failed(now_ms);
    }
}

/// One tick of the keypad's power countdown: the ticks left and whether the
/// keypad turns its signal off now. Upstream schedules a block tick at
/// `signalLength` when it powers on and clears `POWERED` there.
#[must_use]
pub fn tick_signal(remaining: i32) -> (i32, bool) {
    if remaining <= 0 {
        return (0, false);
    }
    let next = remaining - 1;
    (next, next == 0)
}

/// Whether a passcode container opened, and if not, why.
///
/// Upstream's `IPasscodeProtected` blocks (`keypad_barrel`, `keypad_chest`,
/// `keypad_furnace`, `keypad_smoker`) reuse the keypad's code check before the
/// block's own container menu may open. The owner check is the same
/// `IOwnable.isOwnedBy` the set and module paths already use; the digest is the
/// same salted SHA-256, so a saved container's code still opens it after a
/// restart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerAccess {
    /// The owner typed the right code: open the container's menu.
    Open,
    /// The player is not the container's owner.
    NotOwner,
    /// The container has no stored code yet.
    NoCode,
    /// A code was stored, but the entry did not match it.
    WrongCode,
}

/// The pure access rule of a carried passcode container.
#[must_use]
pub fn container_access(
    owner: Option<u128>,
    player: u128,
    stored: Option<&str>,
    salt_hex: &str,
    entered: &str,
) -> ContainerAccess {
    if !may_set_code(owner, player) {
        return ContainerAccess::NotOwner;
    }
    let Some(stored) = stored else {
        return ContainerAccess::NoCode;
    };
    match submit(entered, salt_hex, Some(stored)) {
        Verdict::Correct => ContainerAccess::Open,
        _ => ContainerAccess::WrongCode,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        bytes.push(high << 4 | low);
    }
    Some(bytes)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// SHA-256 of `message` (FIPS 180-4), this port's own code.
///
/// The keypad's digest used to come from the `sha2` crate; the registry refuses
/// any package that declares a dependency other than the SDK
/// (`tools/native-subset/src/manifests.rs`, R9), so the hash lives here instead.
/// It is written from the standard — the round constants, the compression
/// function and the padding — not translated from a crate.
///
/// Padding: one `0x80` byte, zeros until the length is 56 modulo 64, then the
/// message's length in bits as a big-endian `u64` — so a message of 55 bytes
/// still fits one block and 56 bytes takes two.
fn sha256(message: &[u8]) -> [u8; 32] {
    let mut state = SHA256_INITIAL;
    let bit_length = (message.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(message.len() + 72);
    padded.extend_from_slice(message);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());
    for block in padded.chunks_exact(64) {
        sha256_compress(&mut state, block);
    }
    let mut digest = [0u8; 32];
    for (word, chunk) in state.iter().zip(digest.chunks_exact_mut(4)) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// The SHA-256 round constants: the first 32 bits of the fractional parts of the
/// cube roots of the first 64 primes (FIPS 180-4, §4.2.2).
const SHA256_K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The SHA-256 initial hash value: the first 32 bits of the fractional parts of
/// the square roots of the first 8 primes (FIPS 180-4, §5.3.3).
const SHA256_INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// One SHA-256 block: extend its sixteen words to sixty-four, then run the
/// sixty-four rounds over the working variables.
fn sha256_compress(state: &mut [u32; 8], block: &[u8]) {
    let mut schedule = [0u32; 64];
    for (word, chunk) in schedule.iter_mut().zip(block.chunks_exact(4)) {
        *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    for index in 16..64 {
        let thirteen_back = schedule[index - 15];
        let two_back = schedule[index - 2];
        let sigma0 =
            thirteen_back.rotate_right(7) ^ thirteen_back.rotate_right(18) ^ (thirteen_back >> 3);
        let sigma1 = two_back.rotate_right(17) ^ two_back.rotate_right(19) ^ (two_back >> 10);
        schedule[index] = schedule[index - 16]
            .wrapping_add(sigma0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(sigma1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (&constant, &word) in SHA256_K.iter().zip(schedule.iter()) {
        let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ (!e & g);
        let temp1 = h
            .wrapping_add(sigma1)
            .wrapping_add(choose)
            .wrapping_add(constant)
            .wrapping_add(word);
        let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = sigma0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }
    for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The salt the in-game path draws is 16 bytes; tests use a fixed one.
    const SALT: &str = "000102030405060708090a0b0c0d0e0f";
    const OTHER_SALT: &str = "ffeeddccbbaa99887766554433221100";

    #[test]
    fn a_code_is_four_to_twenty_ascii_digits() {
        assert!(valid_code("1234"));
        assert!(valid_code("0123"));
        assert!(valid_code(&"9".repeat(MAX_CODE_LEN)));
        assert!(!valid_code(""));
        assert!(!valid_code("0"), "a one-digit code is a ten-value secret");
        assert!(!valid_code("123"), "three digits are still too few");
        assert!(!valid_code(&"9".repeat(MAX_CODE_LEN + 1)));
        assert!(!valid_code("12a4"));
        assert!(!valid_code("12 4"));
        assert!(!valid_code("-123"));
    }

    #[test]
    fn a_too_short_code_is_refused_before_any_comparison() {
        let stored = hash_code("1234", SALT).unwrap();
        for short in ["1", "12", "123"] {
            assert_eq!(
                after_submit(Mode::Set, short, SALT, None),
                Action::TooShort,
                "set mode must refuse {short}"
            );
            assert_eq!(
                after_submit(Mode::Check, short, SALT, Some(&stored)),
                Action::TooShort,
                "check mode must refuse {short} without looking at the digest"
            );
        }
    }

    #[test]
    fn five_wrong_codes_lock_the_session_out_for_ten_seconds() {
        let mut throttle = Throttle::default();
        assert!(throttle.allows(0));
        for attempt in 0..MAX_FAILED_ATTEMPTS {
            assert!(
                throttle.allows(u64::from(attempt)),
                "attempt {attempt} is allowed"
            );
            throttle.failed(u64::from(attempt));
        }
        let locked_at = u64::from(MAX_FAILED_ATTEMPTS - 1);
        assert!(!throttle.allows(locked_at + 1), "the budget is spent");
        assert!(!throttle.allows(locked_at + LOCKOUT_MS - 1));
        assert!(
            throttle.allows(locked_at + LOCKOUT_MS),
            "the pause ends after LOCKOUT_MS"
        );
        // The next five failures after the pause start another lockout.
        for _ in 0..MAX_FAILED_ATTEMPTS {
            throttle.failed(locked_at + LOCKOUT_MS);
        }
        assert!(!throttle.allows(locked_at + LOCKOUT_MS + 1));
    }

    #[test]
    fn a_correct_code_clears_the_budget() {
        let mut throttle = Throttle::default();
        for _ in 0..MAX_FAILED_ATTEMPTS - 1 {
            throttle.failed(0);
        }
        throttle.succeeded();
        for attempt in 0..MAX_FAILED_ATTEMPTS - 1 {
            throttle.failed(u64::from(attempt));
        }
        assert!(
            throttle.allows(100),
            "four failures after a success do not lock"
        );
    }

    #[test]
    fn a_keypad_without_the_smart_module_has_no_cooldown() {
        // Upstream applies a keypad cooldown only when the smart module is installed:
        // `onIncorrectPasscodeEntered` calls `startCooldown()` under `isModuleEnabled(SMART)`.
        let mut throttle = Throttle::default();
        for attempt in 0..MAX_FAILED_ATTEMPTS {
            assert!(
                cooldown_allows(false, &throttle, u64::from(attempt)),
                "attempt {attempt} on a keypad without the module is allowed"
            );
            cooldown_failed(false, &mut throttle, u64::from(attempt));
        }
        assert!(
            cooldown_allows(false, &throttle, u64::from(MAX_FAILED_ATTEMPTS)),
            "five wrong codes must not rate-limit a keypad without the smart module"
        );
        assert_eq!(
            throttle,
            Throttle::default(),
            "no budget is recorded without the module"
        );
    }

    #[test]
    fn the_smart_module_gates_the_existing_retry_budget() {
        let mut throttle = Throttle::default();
        for attempt in 0..MAX_FAILED_ATTEMPTS {
            cooldown_failed(true, &mut throttle, u64::from(attempt));
        }
        let locked = u64::from(MAX_FAILED_ATTEMPTS);
        assert!(
            !cooldown_allows(true, &throttle, locked),
            "the module's fifth wrong code locks the session out"
        );
        assert!(
            cooldown_allows(false, &throttle, locked),
            "the same budget is ignored without the module"
        );
        throttle.succeeded();
        assert!(
            cooldown_allows(true, &throttle, locked),
            "a correct code clears the module's budget"
        );
    }

    #[test]
    fn the_digest_is_sha256_of_the_salt_then_the_code() {
        // sha256(bytes.fromhex(SALT) + b"1234")
        assert_eq!(
            hash_code("1234", SALT).as_deref(),
            Some("bad0b4f7cae08eb7c1b5acc763a8ed25242e2c9173e81f3f624b1fa5303cf6ce")
        );
    }

    #[test]
    fn the_hash_matches_the_standard_vectors() {
        // The empty message and the 55/56-byte padding boundary: 55 bytes leave
        // room for the `0x80` and the 8-byte bit length in one block, 56 do not,
        // so a padding bug shows up on exactly one of them.
        assert_eq!(
            encode_hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "the empty message"
        );
        assert_eq!(
            encode_hex(&sha256(&[b'a'; 55])),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            "55 bytes: one padded block"
        );
        assert_eq!(
            encode_hex(&sha256(&[b'a'; 56])),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            "56 bytes: the bit length does not fit the first block"
        );
        assert_eq!(
            encode_hex(&sha256(&[b'a'; 1000])),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3",
            "1000 bytes: many blocks"
        );
    }

    #[test]
    fn the_same_code_under_another_salt_has_another_digest() {
        assert_ne!(hash_code("1234", SALT), hash_code("1234", OTHER_SALT));
        assert_eq!(
            hash_code("1234", OTHER_SALT).as_deref(),
            Some("3c98a0ee8cde8b39aca8950f017c0e607e1c274706e665c492b6e903f886cf43")
        );
    }

    #[test]
    fn a_different_code_under_the_same_salt_has_another_digest() {
        assert_ne!(hash_code("1234", SALT), hash_code("4321", SALT));
    }

    #[test]
    fn an_empty_code_has_no_digest() {
        assert_eq!(hash_code("", SALT), None);
    }

    #[test]
    fn a_malformed_salt_has_no_digest() {
        assert_eq!(hash_code("1234", "abc"), None);
        assert_eq!(hash_code("1234", "zz"), None);
        assert_eq!(hash_code("1234", ""), Some(hash_code("1234", "").unwrap()));
    }

    #[test]
    fn a_keypad_without_a_code_says_so() {
        assert_eq!(submit("1234", SALT, None), Verdict::NotSet);
    }

    #[test]
    fn an_empty_entry_is_never_correct() {
        assert_eq!(
            submit("", SALT, hash_code("1234", SALT).as_deref()),
            Verdict::Empty
        );
    }

    #[test]
    fn the_right_code_matches() {
        let stored = hash_code("1234", SALT).unwrap();
        assert_eq!(submit("1234", SALT, Some(&stored)), Verdict::Correct);
    }

    #[test]
    fn any_other_code_is_incorrect() {
        let stored = hash_code("1234", SALT).unwrap();
        assert_eq!(submit("4321", SALT, Some(&stored)), Verdict::Incorrect);
        assert_eq!(submit("123", SALT, Some(&stored)), Verdict::Incorrect);
        assert_eq!(submit("12345", SALT, Some(&stored)), Verdict::Incorrect);
    }

    #[test]
    fn a_digest_from_another_salt_does_not_match() {
        let stored = hash_code("1234", OTHER_SALT).unwrap();
        assert_eq!(submit("1234", SALT, Some(&stored)), Verdict::Incorrect);
    }

    #[test]
    fn entry_collects_digits_in_order() {
        let mut entry = Entry::new();
        assert!(entry.push(1));
        assert!(entry.push(2));
        assert_eq!(entry.text(), "12");
        assert_eq!(entry.len(), 2);
        assert!(!entry.is_empty());
    }

    #[test]
    fn entry_stops_at_the_code_limit() {
        let mut entry = Entry::new();
        for digit in 0..MAX_CODE_LEN {
            assert!(entry.push(u8::try_from(digit % 10).unwrap()));
        }
        assert!(!entry.push(7));
        assert_eq!(entry.len(), MAX_CODE_LEN);
    }

    #[test]
    fn entry_refuses_a_non_digit() {
        let mut entry = Entry::new();
        assert!(!entry.push(10));
        assert!(entry.is_empty());
    }

    #[test]
    fn backspace_removes_the_last_digit_only() {
        let mut entry = Entry::new();
        entry.push(1);
        entry.push(2);
        assert!(entry.backspace());
        assert_eq!(entry.text(), "1");
        assert!(entry.backspace());
        assert!(!entry.backspace());
        assert!(entry.is_empty());
    }

    #[test]
    fn clear_forgets_everything() {
        let mut entry = Entry::new();
        entry.push(4);
        entry.push(2);
        entry.clear();
        assert!(entry.is_empty());
    }

    #[test]
    fn the_display_hides_the_digits() {
        let mut entry = Entry::new();
        entry.push(9);
        entry.push(1);
        entry.push(2);
        assert_eq!(entry.masked(), "***");
        assert_eq!(Entry::new().masked(), "");
    }

    #[test]
    fn the_signal_counts_down_and_turns_off_once() {
        assert_eq!(tick_signal(3), (2, false));
        assert_eq!(tick_signal(2), (1, false));
        assert_eq!(tick_signal(1), (0, true));
        assert_eq!(tick_signal(0), (0, false));
        assert_eq!(tick_signal(-4), (0, false));
    }

    #[test]
    fn the_screen_keys_map_to_digits_and_the_two_actions() {
        assert_eq!(button(0), Some(Button::Digit(0)));
        assert_eq!(button(9), Some(Button::Digit(9)));
        assert_eq!(button(BUTTON_BACKSPACE), Some(Button::Backspace));
        assert_eq!(button(BUTTON_SUBMIT), Some(Button::Submit));
        assert_eq!(button(12), None);
        assert_eq!(button(-1), None);
    }

    #[test]
    fn the_keypad_faces_the_player_who_places_it() {
        assert_eq!(facing_from_yaw(0.0), "north");
        assert_eq!(facing_from_yaw(90.0), "east");
        assert_eq!(facing_from_yaw(180.0), "south");
        assert_eq!(facing_from_yaw(-90.0), "west");
        assert_eq!(facing_from_yaw(360.0), "north");
    }

    #[test]
    fn modes_round_trip_through_the_saved_int() {
        assert_eq!(Mode::from_int(1), Mode::Set);
        assert_eq!(Mode::from_int(0), Mode::Check);
        assert_eq!(Mode::from_int(7), Mode::Check);
        assert_eq!(Mode::Set.code(), 1);
        assert_eq!(Mode::Check.code(), 0);
    }

    #[test]
    fn set_mode_stores_a_valid_code() {
        assert_eq!(after_submit(Mode::Set, "2468", SALT, None), Action::Set);
    }

    #[test]
    fn set_mode_with_nothing_typed_does_nothing() {
        assert_eq!(after_submit(Mode::Set, "", SALT, None), Action::Empty);
    }

    #[test]
    fn checking_an_unset_keypad_switches_to_set() {
        assert_eq!(after_submit(Mode::Check, "1234", SALT, None), Action::Check);
    }

    #[test]
    fn a_matching_code_opens_the_door() {
        let stored = hash_code("1234", SALT).unwrap();
        assert_eq!(
            after_submit(Mode::Check, "1234", SALT, Some(&stored)),
            Action::Open
        );
    }

    #[test]
    fn a_wrong_code_does_nothing() {
        let stored = hash_code("1234", SALT).unwrap();
        assert_eq!(
            after_submit(Mode::Check, "9999", SALT, Some(&stored)),
            Action::Wrong
        );
    }

    #[test]
    fn enter_with_nothing_typed_is_empty() {
        let stored = hash_code("1234", SALT).unwrap();
        assert_eq!(
            after_submit(Mode::Check, "", SALT, Some(&stored)),
            Action::Empty
        );
    }

    #[test]
    fn a_salt_round_trips_through_the_saved_hex() {
        let bytes = [0x00, 0x0f, 0xa5, 0xff];
        assert_eq!(new_salt(&bytes), "000fa5ff");
    }

    #[test]
    fn a_harming_module_damages_a_wrong_code() {
        assert_eq!(
            wrong_code_damage(true),
            Some(4.0),
            "two hearts, as upstream's incorrectPasscodeDamage default"
        );
        assert_eq!(
            wrong_code_damage(false),
            None,
            "a keypad without the module only reports the failure"
        );
    }

    #[test]
    fn only_the_keypad_owner_may_install_a_module() {
        let owner = Some(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef_u128);
        assert!(may_install(
            owner,
            0x0123_4567_89ab_cdef_0123_4567_89ab_cdef
        ));
        assert!(
            !may_install(owner, 0xdead_beef_dead_beef_dead_beef_dead_beef),
            "a stranger installs nothing"
        );
        assert!(
            !may_install(None, 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef),
            "an unowned keypad (old save, dispenser) installs nothing"
        );
    }

    #[test]
    fn an_allowed_player_activates_without_a_code() {
        let list = ListData::new().with_player("Alice");
        assert!(
            is_allowed(true, &list, "alice"),
            "a listed player is allowed without typing a code"
        );
        assert!(
            !is_allowed(true, &list, "stranger"),
            "a player who is not on the list still needs the code"
        );
        assert!(
            !is_allowed(false, &list, "alice"),
            "without the module the list grants nothing"
        );
        assert!(
            is_allowed(true, &ListData::new().affecting_everyone(true), "anyone"),
            "a list set to affect everyone allows a player who is not named"
        );
    }

    #[test]
    fn the_allowlist_is_bounded_and_case_insensitive() {
        let mut list = ListData::new();
        for index in 0..MAX_LIST_PLAYERS {
            list = list.with_player(&format!("player{index}"));
        }
        list = list.with_player("one-too-many");
        assert!(
            !list.contains("one-too-many"),
            "the list holds at most MAX_LIST_PLAYERS players"
        );
        assert!(
            list.contains("PLAYER49"),
            "upstream compares player names case-insensitively"
        );
        assert!(
            ListData::new()
                .with_player("alice")
                .with_player("alice")
                .contains("alice"),
            "adding the same player twice keeps one entry"
        );
    }

    #[test]
    fn a_denied_player_is_refused() {
        let list = ListData::new().with_player("Alice");
        assert!(
            is_denied(true, &list, "alice"),
            "a listed player is denied, upstream's ListModuleData.isPlayerOnList"
        );
        assert!(
            !is_denied(true, &list, "stranger"),
            "a player who is not on the list may still interact"
        );
        assert!(
            !is_denied(false, &list, "alice"),
            "without the module the list denies nobody"
        );
        assert!(
            is_denied(true, &ListData::new().affecting_everyone(true), "anyone"),
            "a list set to affect everyone denies a player who is not named"
        );
    }

    #[test]
    fn the_denylist_is_bounded_and_case_insensitive() {
        let mut list = ListData::new();
        for index in 0..MAX_LIST_PLAYERS {
            list = list.with_player(&format!("banned{index}"));
        }
        list = list.with_player("one-too-many");
        assert!(
            !list.contains("one-too-many"),
            "the denylist holds at most MAX_LIST_PLAYERS players"
        );
        assert!(
            list.contains("BANNED49"),
            "denylist names compare case-insensitively like allowlist names"
        );
    }

    #[test]
    fn a_stranger_may_not_set_the_first_code() {
        let owner = Some(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef_u128);
        assert!(
            may_set_code(owner, 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef),
            "the owner sets the first code"
        );
        assert!(
            !may_set_code(owner, 0xdead_beef_dead_beef_dead_beef_dead_beef),
            "a stranger sets nothing, as upstream's verifyPasscodeSet refuses it"
        );
        assert!(
            !may_set_code(None, 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef),
            "an unowned keypad (old save, dispenser) is nobody's to set"
        );
    }

    #[test]
    fn a_passcode_container_opens_for_its_owner_with_the_code() {
        let owner = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef_u128;
        let stranger = 0xdead_beef_dead_beef_dead_beef_dead_beef_u128;
        let digest = hash_code("1234", SALT).unwrap();
        assert_eq!(
            container_access(Some(owner), owner, Some(&digest), SALT, "1234"),
            ContainerAccess::Open,
            "the owner with the right code opens the container"
        );
        assert_eq!(
            container_access(Some(owner), owner, Some(&digest), SALT, "9999"),
            ContainerAccess::WrongCode,
            "a wrong code leaves a passcode container closed and its contents unreadable"
        );
        assert_eq!(
            container_access(Some(owner), stranger, Some(&digest), SALT, "1234"),
            ContainerAccess::NotOwner,
            "the code alone does not open another player's container"
        );
        assert_eq!(
            container_access(Some(owner), owner, None, SALT, "1234"),
            ContainerAccess::NoCode,
            "a container with no stored code cannot be checked"
        );
    }

    #[test]
    fn a_disguised_keypad_tints_only_with_the_module() {
        // Upstream's disguise module makes the device render as another block. This port
        // fixes one distinctive tint; a keypad without the module keeps its own look.
        assert_eq!(disguise_tint(&[]), None);
        assert_eq!(
            disguise_tint(&["securitycraft:harming_module".to_owned()]),
            None,
            "another installed module must not disguise the keypad"
        );
        assert_eq!(
            disguise_tint(&[
                "securitycraft:harming_module".to_owned(),
                DISGUISE_MODULE.to_owned(),
            ]),
            Some(DISGUISE_TINT),
            "the disguise module tints the keypad even beside other modules"
        );
    }
}
