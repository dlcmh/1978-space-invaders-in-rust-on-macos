# Q&A

Questions about how **SPACE INVADERS — Metal Arcade** works, answered from the
source. Each entry is a standalone document named after the question and cites
the files and symbols that implement the behavior, so references survive
unrelated edits.

Documents are listed newest first, by the **Last updated** date recorded in each
file. When dates are equal, the most recently added question comes first.

| Question | Last updated | Covers |
| --- | --- | --- |
| [When would sprite sheets, a game engine, texture assets, or runtime downloads be needed?](procedural-assets.md) | 2026-10-20 | Procedural art, the escalation ladder, and the triggers that justify an asset pipeline |
| [How does the game make sound without audio files?](audio-synth.md) | 2026-10-20 | `AudioQueue`, the voice bank, envelopes, and event-to-cue mapping |
| [How does one draw call render the whole game?](metal-renderer.md) | 2026-10-20 | Canvas triangles, triple buffering, MSAA, uniforms, and presentation |
| [How does the invader march and collision system work?](formation-and-collisions.md) | 2026-10-20 | Formation stepping, acceleration, bullet order, bunker craters |
| [How is the Rust workspace split between simulation and presentation?](architecture.md) | 2026-10-20 | The core/app boundary, commands, snapshots, events, and determinism |

## Adding an entry

1. Name the file after the question, not the date (`formation-and-collisions.md`),
   so links survive re-edits.
2. Start with the question as the `#` heading, then record both a `Recorded` and
   a `Last updated` date in the metadata block, matching the existing entries.
3. Insert the document into the table above in descending date order.
4. Cite code by file and symbol, never by line number, and run `make docs-check`
   before committing. The convention is defined in
   [`AGENTS.md`](../../AGENTS.md#documentation-and-code-references).

To confirm the date to record, use the commit that last touched the file:

```bash
git log -1 --format=%cs -- Docs/QnA/<file>.md
```

The in-file date is authoritative because it survives cloning, where file
modification times do not. The two should agree with the command above.
