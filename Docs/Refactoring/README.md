# Refactoring and Design Decisions

Dated records of architectural choices that were proposed and evaluated. Each
entry captures the evidence behind the decision — dependencies, call sites, and
the constraints — so a rejected idea does not have to be re-litigated from
memory, and an accepted one has a stated rationale to check the code against.

Documents are listed newest first, by the **Last updated** date recorded in each
file.

| Document | Last updated | Decision |
| --- | --- | --- |
| [Direct `objc2` bindings instead of `winit` + `metal-rs`](objc2-over-winit-metal-rs.md) | 2026-10-20 | Accepted — one platform story, native AppKit menus and focus, smaller dependency surface |

## Adding an entry

1. Name the file after the change, not the date
   (`objc2-over-winit-metal-rs.md`), so links survive re-edits.
2. Record both a `Recorded` and a `Last updated` date in the document's metadata
   block, and keep them current.
3. Insert the document into the table above in descending date order.
4. Cite code by file and symbol, never by line number, and run `make docs-check`
   before committing. The convention is defined in
   [`AGENTS.md`](../../AGENTS.md#documentation-and-code-references).

To confirm the date to record, use the commit that last touched the file:

```bash
git log -1 --format=%cs -- Docs/Refactoring/<file>.md
```

The in-file date is authoritative because it survives cloning, where file
modification times do not. The two should agree with the command above.
