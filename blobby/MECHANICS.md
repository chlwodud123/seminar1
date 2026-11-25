# Game Mechanics Specification (Disassembled from `blobby.exe`)

All constants, formulas, and rules below were extracted directly by unpacking `C:\Program Files (x86)\Blobby Volley 2 Version 1.1.1\blobby.exe` (UPX 4.02, NRV2E_LE32 stub at `0x004E1EA0`, 914,818 bytes uncompressed at base `0x00401000`) and disassembling its x86 code with `llvm-objdump`, cross-checked against `data/config.xml`, `data/inputconfig.xml`, `data/api.lua`, `data/bot_api.lua`, and `data/rules.zip`.

> Note: Craft Anything MCP decompilation tools (`core_target_open`, `native_strings_find`, `native_function_pseudocode`) were not exposed in this session's tool list, so disassembly was performed directly on a read-only unpacked copy in scratch storage.

## Key Functions in `blobby.exe` (x86, ImageBase `0x00400000`)
- `0x004129A0` — `PhysicWorld::PhysicWorld()`: initializes ball `(200.0, 300.5)`, `ball_angular_vel = 0.1`, left blob `(200.0, 455.5)`, right blob `(600.0, 455.5)`.
- `0x00412AF0` — `PhysicWorld::blobHitGround(side)`: checks `blob.y >= 455.5` (`0x004897C8`).
- `0x00412C50` — `PhysicWorld::handleBallWorldCollisions()`: ground, left/right wall, vertical net shaft, and net top sphere collisions.
- `0x00413090` — `PhysicWorld::handleBlob(side, input)`: horizontal movement, jump takeoff, jump-hold gravity buffer, Verlet vertical integration, and blob animation state.
- `0x00413260` — `PhysicWorld::playerBallCollision(side)`: dual-sphere blob-ball collision (lower sphere then upper sphere) and collision velocity redirection.
- `0x004135B0` — `PhysicWorld::step(...)`: invokes `handleBlob`, Verlet ball integration, `playerBallCollision`, `handleBallWorldCollisions`, horizontal clamping, and ball angular rotation.
- `0x00410C80` — `GameLogic::onBallHitsPlayerLua` (`OnBallHitsPlayer` xref `0x00410C89`): checks `touches[side] > 3` (`cmp [esi+4*eax+0x10], 3`).
- `0x004119E0` — `GameLogic::onBallHitsGround`: sets ground squish cooldown `11` (`0x0B` at `0x004119F2`).
- `0x00411A30` — `GameLogic::onBallHitsNet`: sets net squish cooldown `11` (`0x0B` at `0x00411A3E`).
- `0x00411A50` — `GameLogic::onBallHitsPlayer`: sets player squish cooldown `11` (`0x0B` at `0x00411A73`), resets opponent squish to `0` (`0x00411A97`), increments touches, sets `is_game_running = true`.
- `0x00411B00` — `GameLogic::onBallHitsWall`: sets wall squish cooldown `11` (`0x0B` at `0x00411B0E`).
- `0x00411BA0` — `GameLogic::step`: decrements all 4 squish timers (`+0x18, +0x1C, +0x20, +0x24`) every tick.

## 1. Court Dimensions & Float Table (`0x00489748..0x004897DC`)
- **Field width**: `800.0` px (`0x004897D0`, used in `0x00412D69`, `0x004136B8`)
- **Field height**: `600.0` px (`data/backgrounds.zip:strand2.bmp`)
- **Ground plane Y**: `500.0` px (`0x004897CC`, used in `0x00412C7E`)
- **Net center X**: `400.0` px (`0x004897BC`, used in `0x00412C69`, `0x0041300A`)
- **Net left/right face X**: `393.0` / `407.0` px (`0x004897B8` / `0x004897C0`, radius `7.0` px)
- **Net top sphere Y**: `284.0` px (`0x004897B0`, used in `0x00412DC2`, `0x00413032`)
- **Ball + net combined radius**: `38.5` px (`0x004897AC`, `-38.5` at `0x004897DC`, used in `0x00412DCD`, `0x00412E91`)

## 2. Player (Blob) Geometry & Movement (`0x00413090`, `0x00413260`, `0x004135B0`)
- **Blob start positions**: Left `(200.0, 455.5)` (`0x00412A38`), Right `(600.0, 455.5)` (`0x00412A4D`)
- **Standing ground center Y**: `455.5` px (`0x004897C8`, `0x43E3C000` at `0x00412A3E`, `0x004131FA`)
- **Upper collision sphere**: offset `-19.0` px (`0x0048979C`), radius `25.0` px (`0x004897A0`), `min_dist^2 = 3192.25` (`0x004897D4`, used in `0x004132DA`)
- **Lower collision sphere**: offset `+13.0` px (`0x00489798`), radius `33.0` px (`0x004897A8`), `min_dist^2 = 4160.25` (`0x004897D8`, used in `0x004132AE`)
- **Horizontal speed**: `4.5` px/tick (`0x00489788`, used in `0x00413120`)
- **Jump takeoff velocity**: `-15.1` px/tick (`0xC171999A` immediate at `0x004130F0`)
- **Blob gravity (no jump held)**: `0.9152434` px/tick² (`0x00489764`, used in `0x0041309F`)
- **Blob gravity (jump held)**: `0.4576217` px/tick² (`0x0048975C`, used in `0x004130BE`, `0x0041310F`)
- **Vertical integration formula** (`0x004131AF..0x004131CF`):
  - `blob.y += blob.vy + 0.5 * grav` (`0.5` at `0x00489760`)
  - `blob.vy += grav`
  - If `blob.y > 455.5`: `blob.y = 455.5`, `blob.vy = 0.0`
- **Horizontal bounds** (`0x00413658..0x004136C8`):
  - Left blob X clamped to `[0.0, 360.0]` (`0x004897B4`)
  - Right blob X clamped to `[440.0, 800.0]` (`0x004897C4`, `0x004897D0`)

## 3. Ball Physics & Collisions (`0x00412C50`, `0x00413260`, `0x004135B0`)
- **Ball radius**: `31.5` px (`0x004897A4`, `0x41FC0000` at `0x00412D33`)
- **Ball serve position**: `(200.0, 300.5)` (`0x43480000`, `0x43964000` at `0x004129D3..0x004129DA`; mirrored `600.0` for right serve)
- **Ball gravity**: `0.287` px/tick² (`0x00489758`), half-step `0.1435` (`0x00489748`)
- **Ball Verlet integration** (`0x004135F9..0x00413634`):
  - `ball.x += ball.vx`
  - `ball.y += ball.vy + 0.1435`
  - `ball.vy += 0.287`
- **Player-ball collision** (`0x00413260`):
  - Tests lower sphere `(blob.x, blob.y + 13.0)` (`dist^2 < 4160.25`) first, then upper sphere `(blob.x, blob.y - 19.0)` (`dist^2 < 3192.25`).
  - `n = normalize(ball_pos - sphere_center)`
  - `ball_vel = n * BALL_COLLISION_VELOCITY` (`13.1225`, loaded from `.data` `0x004A074C` at `0x00413409`)
  - `ball_pos += ball_vel` (`0x00413430..0x0041344A`)
- **Ground collision** (`0x00412C75..0x00412CE8`):
  - If `ball.y + 31.5 > 500.0`: `ball.vx *= 0.95`, `ball.vy = -ball.vy * 0.95` (`0.95` at `0x00489768`), `ball.y = 468.5` (`0x43EA4000`).
- **Wall collisions** (`0x00412D03..0x00412DB8`):
  - Left (`ball.x - 31.5 <= 0.0 && ball.vx < 0.0`): `ball.vx = -ball.vx` (elastic `1.0`), `ball.x = 31.5`.
  - Right (`ball.x + 31.5 >= 800.0 && ball.vx > 0.0`): `ball.vx = -ball.vx` (elastic `1.0`), `ball.x = 768.5` (`0x44402000`).
- **Net vertical post collision** (`0x00412DBD..0x00412E41`):
  - If `ball.y > 284.0 && |ball.x - 400.0| < 38.5`: `ball.vx = -ball.vx` (elastic `1.0`), `ball.x = 400.0 +/- 38.5`.
- **Net top sphere collision** (`0x00412E46..0x0041306B`):
  - Let `n = normalize((400.0, 284.0) - ball_pos)`, `v_par = dot(v, n)`, `v_perp_sq = |v|^2 - v_par^2`.
  - `new_speed = sqrt(v_perp_sq * 0.9 + v_par^2 * 0.7)` (`f64 0.9` at `0x00489778`, `f64 0.7` at `0x00489770`).
  - `v = normalize(v - 2 * v_par * n) * new_speed`, `ball_pos = (400.0, 284.0) - n * 38.5`.
- **Ball angular rotation** (`0x004136C8..0x00413796`):
  - Base angular velocity `0.1` (`0x3DCCCCCD` at `0x00412A12`).
  - Idle: `rot -= angular_vel`. Moving: `delta = (|v| / 6.0) * angular_vel` (`6.0` at `0x00489790`), added if `vx > 0` else subtracted, wrapped to `[0.0, 6.25)` (`6.25` at `0x00489794`).

## 4. Rules, Cooldowns & Timing
- **Squish cooldown**: `11` ticks (`0x0B` in `0x004119F2`, `0x00411A3E`, `0x00411A73`, `0x00411B0E`)
- **Max touches per side**: `3` (`0x00410CAF`, `rules/default.lua`)
- **Simulation tick rate**: `75` Hz (`data/config.xml` `gamefps="75"`, xref `0x00405AD5`)
- **Score to win**: `15` points, win by `2` (`data/config.xml` `scoretowin="15"`, xref `0x004093F1`, `rules/default.lua`)
- **Post-point reset delay**: `105` ticks (*estimated* — visual reset pause before serving next rally)
- **Game speed menu presets**: Very Slow `40%`, Slow `75%`, Default `100%`, Fast `125%`, Very Fast `150%` (*estimated* UI multipliers)
