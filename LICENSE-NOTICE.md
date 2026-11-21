# Licence notice — SecurityCraft 1.10.2.1 (keypad + reinforced door)

- **Upstream:** SecurityCraft 1.10.2.1, <https://github.com/Geforce132/SecurityCraft>
  (`1.21.1` branch, commit `c4d35b0c8bf85c2a94c37a1f8ce37ef5ce1a6aa8`).
- **Authors:** Geforce, bl4ckscor3, Redstone_Dubstep, and ChainmailPickaxe
  (the jar's `META-INF/neoforge.mods.toml` `authors` line, reproduced exactly).
- **Licence as read:** MIT — the jar's `META-INF/neoforge.mods.toml` declares
  `license = "MIT https://github.com/Geforce132/SecurityCraft/blob/master/LICENSE"`
  (`ports/_shared/licence-cache/`), and the upstream repository's `LICENSE` at
  the pinned commit is the MIT text whose copyright line is, verbatim:

  ```
  Copyright (c) 2025 The SecurityCraft development team
  ```

  Licence rank 0 (permissive).
- **Complete licence text:** [`LICENSE`](LICENSE), which keeps that original
  copyright line **verbatim** beside the port's own.
- **What travels with the artifact:** `LICENSE` and this notice are packaged
  **inside** `securitycraft.hmod`, so an artifact-only download carries them
  (the MIT notice condition).
- **What the port is:** a clean-room Rust rewrite of the mod's behaviour against
  Horizon's mod API. The carried assets (blockstates, models, textures, lang,
  loot tables, the key panel recipe and the `mineable/pickaxe` tag) are the
  upstream mod's own, reproduced under the same MIT licence.
- **Status:** local-only until the maintainer approves publishing
  (ARCHITECTURE.md §8).
