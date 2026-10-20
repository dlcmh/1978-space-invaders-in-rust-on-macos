# How does the game make sound without audio files?

- **Recorded:** 2026-10-20
- **Last updated:** 2026-10-20

## Short answer

A sixteen-voice software synthesiser generates 44.1 kHz mono samples inside an
`AudioQueue` callback. Gameplay emits semantic events; the app maps each event to
a [`SoundCue`](../../crates/space-invaders-app/src/audio.rs) and triggers voices with a
waveform, frequency sweep, duration, amplitude, and optional delay. No audio file
is read, and no sample data is committed to the repository.

## The output path

[`AudioEngine::new`](../../crates/space-invaders-app/src/audio.rs) describes a linear
PCM stream to `AudioToolbox`:

- 44,100 Hz;
- one channel;
- 16-bit signed integers, packed;
- 8,192-byte buffers, four of them.

The format is built as an `AudioStreamBasicDescription`. Four buffers are
allocated, zero-filled, and enqueued before `AudioQueueStart`. When the queue
finishes a buffer it calls `audio_callback`
([`audio.rs`](../../crates/space-invaders-app/src/audio.rs)) (`audio_callback`), which
fills it with the next block of samples and enqueues it again.

The callback runs on an AudioToolbox thread. It receives a pointer to the
`Arc<Mutex<Synth>>` installed at startup, locks it for one buffer fill, and writes
interleaved samples through a raw pointer. The pointer and buffer lifetime are
guaranteed by the AudioQueue contract and are documented with `SAFETY` comments.

## The voice

Each [`Voice`](../../crates/space-invaders-app/src/audio.rs) holds:

- a waveform (square, triangle, saw, or noise);
- a phase accumulator;
- start and end frequencies;
- elapsed and total sample counts;
- a delay before the voice starts;
- amplitude and attack length;
- a noise state for the noise waveform.

[`Voice::sample`](../../crates/space-invaders-app/src/audio.rs) advances the phase by
`frequency / sample_rate`, computes the selected waveform, applies a short attack
and a linear release, and returns one float sample. When the sample count reaches
the duration, the voice deactivates itself.

Frequency sweeps are linear between the start and end values. This is how the
player's shot falls in pitch and the mystery ship's tone rises.

## The cue map

The game does not know about waveforms. It emits
[`GameEvent`](../../crates/space-invaders-core/src/game.rs) (`GameEvent`) values from
the simulation.
[`dispatch_event`](../../crates/space-invaders-app/src/view.rs) maps those to cues:

| Event | Cue | Character |
| --- | --- | --- |
| `AlienStep` | `March(step)` | Four descending square notes, one per step |
| `PlayerShoot` | `PlayerShoot` | Fast square sweep down |
| `EnemyShoot` | `EnemyShoot` | Triangle sweep down |
| `InvaderKilled` | `InvaderDeath` | Noise burst plus a falling square echo |
| `PlayerHit` | `PlayerDeath` | Long saw sweep and delayed noise |
| `MysteryAppeared` | `MysteryShip` | Rising triangle with a bright wobble |
| `MysteryKilled` | `MysteryDeath` | Noise plus a fast descending zap |
| `ExtraLife` | `ExtraLife` | Four-note ascending arpeggio |
| `WaveCleared` | `WaveClear` | Four-note triangle flourish |
| `GameOver` | `GameOver` | Four descending saw notes |

The marching step counter lives in the app (`march_step`), not in the synthesiser,
because it is presentation state: two identical simulations could be muted or
unmuted and should not diverge.

## Mixing and voice stealing

[`Synth::render`](../../crates/space-invaders-app/src/audio.rs) sums every active voice,
clamps the result to −1.0…1.0, scales it, and converts to `i16`. Muting writes
silence but still advances the voices, so unmuting does not replay a backlog.

[`Synth::next_voice`](../../crates/space-invaders-app/src/audio.rs) uses the first free
voice and steals the voice closest to completion when all sixteen are busy. A
sixteen-voice ceiling is generous for this game: the loudest cue (extra life) uses
four voices, and overlapping effects rarely exceed eight.

## Why a mutex is acceptable

The game thread triggers a cue only on a key press or a gameplay event; the audio
thread locks once per 93 ms buffer. The critical sections are tiny and there is no
lock held across I/O. A lock-free ring buffer would be the natural next step if
the synth grew expensive, but it would add complexity without improving this
workload.

The callback does allocate nothing. It renders into a fixed stack buffer and
writes it to the AudioQueue buffer. Rust's standard library sync primitives keep
the code portable within the macOS target.

## Failure behaviour

[`AudioEngine::new`](../../crates/space-invaders-app/src/audio.rs) returns an error if
`AudioQueueNewOutput`, buffer allocation, or start fails. The app stores
`Option<AudioEngine>` and continues without audio. The mute preference remains
usable, and the game does not crash because a machine has no output device.

On quit, [`AudioEngine`](../../crates/space-invaders-app/src/audio.rs) stops the queue,
disposes it synchronously, and then reclaims the boxed `Arc` that was passed to
the callback. That ordering prevents the callback from touching freed state.
