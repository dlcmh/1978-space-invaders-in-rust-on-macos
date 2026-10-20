# How do I watch CPU and memory usage while playing the game?

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20

## Short answer

Two WezTerm panes. Run the game in one, and follow its process with `top` in the
other:

```bash
open dist/SPACE-INVADERS.app
pid=""
for _ in {1..20}; do
  pid=$(pgrep -f "SPACE-INVADERS.app/Contents/MacOS/SpaceInvaders" | head -1)
  [ -n "$pid" ] && break
  sleep 0.5
done
[ -n "$pid" ] && top -pid "$pid" -stats pid,cpu,mem,threads,time,state \
             || echo "no process found — app did not start"
```

Split panes with `Option+Ctrl+"` or `Option+Ctrl+%` (WezTerm defaults; the `%`
one is the horizontal split). `q` quits `top`.

## Why the obvious one-liner fails

This is a common first attempt:

```bash
open dist/SPACE-INVADERS.app && sleep 2 && top -pid "$(pgrep -x SpaceInvaders | head -1)" ...
```

It fails with `invalid -pid argument (not an integer):` when `pgrep` finds
nothing, because `open` is asynchronous: it returns success as soon as macOS
accepts the request, long before the process exists. The `&&` chain then runs
`top` with an empty pid, and `top` prints its usage text. `sleep 2` is a guess,
not a synchronisation primitive.

Three things commonly leave the pid empty:

- The app is still launching (usually under a second, but not guaranteed).
- A Gatekeeper prompt is waiting for approval, so nothing was launched.
- The app crashed on launch; check `~/Library/Logs/DiagnosticReports`.

Matching the full binary path with `pgrep -f` is also more reliable than
matching the bare process name with `pgrep -x`. The bundle's executable is
`SpaceInvaders`, declared as `CFBundleExecutable` in the bundle's `Info.plist`.

## Reading the numbers

| Column | Meaning |
| --- | --- |
| `%CPU` | Processor time as a percentage of one core. `100` means one core saturated; macOS can report more than 100 for a multi-threaded process. |
| `MEM` | Resident set size — the physical memory the process currently occupies. Activity Monitor's "Memory" column uses a different accounting and will not match exactly. |
| `#TH` | Thread count. The game runs the simulation on the main thread, keeps a Metal driver thread, and adds the audio queue's thread. |
| `TIME` | Total CPU time consumed since launch, in `MM:SS.hh`. |
| `STATE` | `running`, `sleeping`, `woke`, and so on. A vsynced game spends most of its life `sleeping`. |

On the machine where this was recorded, a freshly launched copy sitting in the
background reported `0.0%` CPU, about `120M` resident in `top`, and 13 threads.
`ps` reports a smaller RSS for the same process, because the two tools account
for shared pages differently; compare like with like. These are baselines, not
specifications: memory tracks window size and GPU back buffers, and CPU depends
on what is being drawn.

The frame loop explains the shape of the numbers.
[`frame_tick`](../../crates/space-invaders-app/src/view.rs) measures elapsed
time and runs up to eight fixed 1/60-second simulation ticks per frame, so a
stall produces catch-up work rather than a time jump.

Rendering happens on every display refresh regardless of simulation state,
through [`draw`](../../crates/space-invaders-app/src/renderer.rs), which presents
the 4× MSAA target created by
[`ensure_multisample`](../../crates/space-invaders-app/src/renderer.rs).

Sound is synthesised in the audio callback by
[`Synth`](../../crates/space-invaders-app/src/audio.rs), on its own thread.

## What is normal, and what is not

Expect single-digit to low-double-digit `%CPU` during play, with short spikes
when many invaders are alive or a wave transitions. Things worth investigating:

- **`MEM` climbs steadily across waves and never comes back down.** The snapshot
  types are copyable values and the vertex buffers are reused, so a monotonic
  climb suggests a leak, not normal churn.
- **`%CPU` pinned near or above 100 on the attract screen.** The attract phase
  still runs the simulation, but it is cheap; a pegged core means something is
  spinning or the driver is retrying.
- **`%CPU` drops to zero while paused.** Pause should freeze the simulation, not
  the renderer. [`TogglePause`](../../crates/space-invaders-core/src/game.rs)
  sets the flag behind [`is_paused`](../../crates/space-invaders-core/src/game.rs),
  and the game keeps drawing the paused frame.
- **Thread count creeping upward.** The audio queue has a fixed thread; the app
  is not meant to spawn threads per frame.

## Other ways to look

```bash
# whole machine, hottest processes first
top -o cpu -n 10

# whole machine, largest memory first
top -o mem -n 10

# one-shot snapshot; RSS is in KiB
ps -o pid,%cpu,rss,etime,comm -p "$(pgrep -f SPACE-INVADERS.app | head -1)"
```

`btop` or `htop` from Homebrew give a friendlier TUI (`f` filters in `btop`,
`F4` in `htop`), but neither is installed by default and the built-in `top`
above needs nothing. Activity Monitor (`open -a "Activity Monitor"`) is the only
easy place to see GPU and energy impact, which `top` does not report.

For sample-level detail rather than headline numbers, Instruments can attach to
the running process:

```bash
xcrun xctrace record --template 'Time Profiler' --attach "$(pgrep -f SPACE-INVADERS.app | head -1)"
```

That writes a `.trace` bundle you open in Instruments. It needs Xcode command
line tools, so it is a debugging step, not a routine check, and it is not part of
[`make check`](../../AGENTS.md#build-run-and-verify).

## Troubleshooting

If the wait loop still reports "no process found":

```bash
pgrep -fl -i invader                                   # anything at all?
ls -l dist/SPACE-INVADERS.app/Contents/MacOS/           # binary present?
xattr -l dist/SPACE-INVADERS.app                        # quarantine flag?
ls -t ~/Library/Logs/DiagnosticReports | head -5         # recent crashes?
```

The bundle is ad-hoc signed, so a copy that arrived with a quarantine attribute
will be blocked until macOS is told to trust it. Approve it once in System
Settings → Privacy & Security, or clear the attribute with
`xattr -d com.apple.quarantine` on the bundle. A locally built bundle from
`make app` normally has no quarantine flag at all.

If the game restarts, the pid changes. Rerun the command, or reuse the path-based
`pgrep -f` line, which never hard-codes a pid.
