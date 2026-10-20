# How does the invader march and collision system work?

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20

## Short answer

The 55 invaders are stored as individual entities with a shared direction and a
single step timer. Every step moves the living formation two pixels sideways; at
a wall it drops eight pixels and reverses. The step interval is a function of how
many invaders remain, which is what makes the march accelerate. Bullets are
one-pixel rectangles resolved in a fixed order so cover always protects what is
behind it.

## The formation

[`Game::build_formation`](../../crates/space-invaders-core/src/game.rs) creates eleven
columns and five rows at 16-pixel spacing. The top row is squid, the middle two
are crab, and the bottom two are octopus. Each alien stores its own position, row,
column, species, and alive flag.

The full set starts at x = 24 and y = 48 in arcade pixels. The initial right edge
is at 198, leaving room to move within the 8–216 playfield.

## Stepping and accelerating

[`Game::step_formation`](../../crates/space-invaders-core/src/game.rs) is called once
per fixed tick. It increments the step timer and compares it with
[`Game::formation_interval`](../../crates/space-invaders-core/src/game.rs).

The interval is derived from the number of living invaders:

```text
base = clamp(alive × 0.58 + 2, 3, 36)
level_speedup = min(level − 1, 6) × 3
interval = max(2, base − level_speedup)
```

At the start, 55 invaders produce an interval of about 34 ticks (roughly half a
second). At one invader the interval is 3 ticks, so the last ship sprints. This
reproduces the *feel* of the 1978 acceleration without claiming cycle accuracy:
the original sped up partly because fewer sprites meant less work for its
processor.

When the timer fires:

1. Find the left and right edges of the living formation.
2. If a two-pixel step would cross the playfield, reverse direction and set the
   vertical drop to eight pixels; otherwise step sideways.
3. Move every living alien, toggle the animation frame, and emit
   [`GameEvent::AlienStep`](../../crates/space-invaders-core/src/game.rs).
4. If any alien's bottom edge reaches the cannon line, end the round.
5. If the formation dropped, crush any bunker cells it now overlaps.

Crushing is why a sufficiently advanced wave can erase a shield that the player
was relying on.

## Collision order

Rectangles and their overlap test live in
[`geometry.rs`](../../crates/space-invaders-core/src/geometry.rs) (`Rect::overlaps`).
The exact order is intentional.

### Player bullet, bottom to top

[`Game::move_player_bullet`](../../crates/space-invaders-core/src/game.rs) does:

1. Move the bullet four pixels upward.
2. Remove it above the HUD.
3. Test every bunker; the first hit carves a crater and consumes the bullet.
4. Test living invaders; the first overlap kills the invader, awards points, and
   creates an explosion.
5. Test the mystery ship.
6. Keep the bullet in flight.

Cover is tested before invaders. A shot that reaches a bunker can never also hit
an invader behind it in the same tick. The same applies to a descending enemy
bullet.

### Enemy bullet, top to bottom

[`Game::move_enemy_bullets`](../../crates/space-invaders-core/src/game.rs) moves each
bullet 1.5 pixels downward and tests, in order:

1. cancellation against the player's single bullet;
2. bunker impact;
3. the player's hitbox;
4. the ground line.

Cancellation is checked before cover because two bullets meeting in mid-air
should spark rather than silently pass through each other.

## Bunker damage

A bunker is a 22 × 16 grid of cells in
[`bunker.rs`](../../crates/space-invaders-core/src/bunker.rs) (`Bunker`). The standard
shape has a rounded roof, a one-cell inset on each side, and an arched doorway
that widens toward the bottom.

[`Bunker::impact`](../../crates/space-invaders-core/src/bunker.rs) converts the
projectile's rectangle into local cell coordinates and chooses the side the
projectile struck: the lowest intact cell for a rising player shot, the highest
for a falling enemy shot. That keeps craters on the visible impact side.

[`Bunker::carve`](../../crates/space-invaders-core/src/bunker.rs) removes an irregular
five-by-seven neighbourhood, with a deterministic random branch that makes the
edge ragged. Damage persists for the rest of the wave because the cells are part
of the simulation, not a renderer effect.

Bunkers are rebuilt at the start of each wave by
[`Game::begin_wave`](../../crates/space-invaders-core/src/game.rs). That is a deliberate
playability choice; it is documented here so no one mistakes it for a bug.

## Mystery ship scoring

[`Game::move_mystery_ship`](../../crates/space-invaders-core/src/game.rs) spawns the
ship after a randomized timer, then moves it at one pixel per tick from a random
side. Its value comes from a deterministic cycle based on the player's shot count:

```text
slot = shots_fired mod 15
slot 14          → 300
slot 10..=13     → 150
slot 5..=9       → 100
otherwise        → 50
```

The original cabinet used a shot-count-dependent table too. This version keeps the
strategy of tracking shots without claiming the exact ROM sequence.

## Why rectangles are enough

Every actor is an axis-aligned rectangle. There are no rotations, slopes, or
circles. The playfield is 224 × 256 pixels and the largest hitbox is a few pixels,
so separating-axis or swept collision would add code without changing outcomes.
Bullet speeds (four and 1.5 pixels per tick) are small relative to the targets
they hit, which keeps tunnelling out of the picture.

The one place where a swept test would matter is the one-pixel bunker grid. The
impact function handles that by scanning the integer cells the projectile's
rectangle touches rather than asking whether its centre is inside a cell.
