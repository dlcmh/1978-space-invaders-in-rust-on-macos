#!/usr/bin/env python3
"""Validate documentation links and Rust code references.

Why this exists
---------------
"game.rs line 312" rots silently: insert ten lines above a function and the
reference still resolves, just to the wrong code. Documentation here cites a
*file plus a symbol name* and the checker proves the symbol still exists.

Rules
-----
1. Every relative link resolves to a file that exists.
2. No ``#L<number>`` line anchors.
3. A ``#fragment`` on a Markdown target must match a real heading.
4. A link into Rust source (``crates/``) or a script (``scripts/``) must be
   followed, in the same sentence or bullet, by a backticked symbol that occurs
   in the linked file.

Usage
-----
    python3 scripts/check_docs.py
    python3 scripts/check_docs.py --changed
    python3 scripts/check_docs.py --changed --base main

``--changed`` reports the documents that reference files changed against the
base commit, plus any cited declaration removed by the diff. It never fails
just because a file changed; validation errors always exit 1.

Opt a single line out with ``<!-- docs-check: ignore -->``.

Standard library only.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CODE_PREFIXES = ("crates/", "scripts/")
CODE_FILES = ("Cargo.toml", "Makefile", "rustfmt.toml")
SKIP_DIRS = {".git", "target", "dist", "Resources", ".build", ".vscode"}

LINK_RE = re.compile(r"(?<!!)\[([^\]]*)\]\(([^)\s]+)\)")
INLINE_CODE_RE = re.compile(r"`([^`]+)`")
LIST_OR_HEADING_RE = re.compile(r"^\s*(#{1,6}\s|[-*+]\s|\d+\.\s|\||```)")
DECL_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?"
    r"(?:unsafe\s+)?(?:async\s+)?(fn|struct|enum|trait|type|const|static|mod|macro_rules!)\s+"
    r"([A-Za-z_][A-Za-z0-9_]*)"
)


class Doc:
    def __init__(self, path: str) -> None:
        self.path = path
        self.lines = open(os.path.join(ROOT, path), encoding="utf-8").read().splitlines()
        self.fenced = [False] * len(self.lines)
        in_fence = False
        for index, line in enumerate(self.lines):
            if line.lstrip().startswith("```"):
                self.fenced[index] = True
                in_fence = not in_fence
            else:
                self.fenced[index] = in_fence


def walk_files() -> list[str]:
    files: list[str] = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = sorted(
            directory
            for directory in dirnames
            if directory not in SKIP_DIRS and not directory.startswith(".")
        )
        for name in sorted(filenames):
            files.append(os.path.relpath(os.path.join(dirpath, name), ROOT))
    return files


def repo_docs() -> list[Doc]:
    return [Doc(path) for path in walk_files() if path.endswith(".md")]


def source_files() -> set[str]:
    return {
        path
        for path in walk_files()
        if path.endswith((".rs", ".sh", ".py"))
    }


def heading_slugs(doc: Doc) -> set[str]:
    slugs: set[str] = set()
    seen: dict[str, int] = {}
    in_fence = False
    for line in doc.lines:
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence or not line.startswith("#"):
            continue
        text = line.lstrip("#").strip()
        if not text:
            continue
        text = re.sub(r"`([^`]*)`", r"\1", text)
        text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
        text = text.lower()
        text = re.sub(r"[^\w\- ]", "", text)
        slug = text.replace(" ", "-")
        count = seen.get(slug, 0)
        seen[slug] = count + 1
        slugs.add(slug if count == 0 else f"{slug}-{count}")
    return slugs


def is_code_target(target: str) -> bool:
    return target.startswith(CODE_PREFIXES) or target in CODE_FILES


def link_context(doc: Doc, index: int, end_column: int) -> str:
    parts = [doc.lines[index][end_column:]]
    cursor = index + 1
    while cursor < len(doc.lines):
        line = doc.lines[cursor]
        if not line.strip() or LIST_OR_HEADING_RE.match(line):
            break
        parts.append(line)
        cursor += 1
    return " ".join(parts)


def cited_symbol(text: str) -> str | None:
    for match in INLINE_CODE_RE.finditer(text):
        raw = match.group(1).strip()
        if raw.endswith((".rs", ".md", ".toml", ".sh", ".py", ".png", ".plist")):
            continue
        if "/" in raw or raw.startswith("#L"):
            continue
        name = raw.split("(")[0].strip().replace("::", ".").split(".")[-1]
        name = name.split("<")[0].strip().rstrip(".,;")
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name or ""):
            return name
    return None


def symbol_in_file(symbol: str, path: str) -> bool:
    try:
        text = open(os.path.join(ROOT, path), encoding="utf-8").read()
    except OSError:
        return False
    return re.search(rf"\b{re.escape(symbol)}\b", text) is not None


def validate() -> tuple[list[str], dict[str, list[tuple[str, str]]]]:
    errors: list[str] = []
    references: dict[str, list[tuple[str, str]]] = {}
    slugs: dict[str, set[str]] = {}
    known_files = set(walk_files())

    for doc in repo_docs():
        for index, line in enumerate(doc.lines):
            if doc.fenced[index]:
                continue
            for match in LINK_RE.finditer(line):
                label, target = match.group(1), match.group(2)
                if target.startswith(("http://", "https://", "mailto:", "#")):
                    continue
                path, _, fragment = target.partition("#")
                where = f"{doc.path}:{index + 1}"
                resolved = (
                    os.path.normpath(os.path.join(os.path.dirname(doc.path), path))
                    if path
                    else doc.path
                )
                if resolved not in known_files:
                    errors.append(f"{where}: link target does not exist: {target}")
                    continue
                if fragment:
                    if re.match(r"L\d+", fragment):
                        errors.append(
                            f"{where}: line anchor '#{fragment}' is not allowed; "
                            "link to the file and cite the symbol"
                        )
                        continue
                    if resolved.endswith(".md"):
                        if resolved not in slugs:
                            slugs[resolved] = heading_slugs(Doc(resolved))
                        if fragment not in slugs[resolved]:
                            errors.append(
                                f"{where}: no heading matches anchor '#{fragment}' in {resolved}"
                            )
                if not is_code_target(resolved):
                    continue
                if "docs-check: ignore" in line:
                    continue
                context = link_context(doc, index, match.end())
                symbol = cited_symbol(context)
                if symbol is None and label.strip() and "`" in label:
                    symbol = cited_symbol(f"`{label}`")
                if symbol is None:
                    errors.append(
                        f"{where}: link to {resolved} must cite a symbol in backticks"
                    )
                    continue
                if not symbol_in_file(symbol, resolved):
                    errors.append(
                        f"{where}: '{symbol}' does not occur in {resolved}; "
                        "the reference has drifted or the symbol was renamed"
                    )
                    continue
                references.setdefault(resolved, []).append((doc.path, symbol))
    return errors, references


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    )
    return result.stdout


def changed_files(base: str) -> list[str]:
    files = [line for line in git("diff", "--name-only", base).splitlines() if line]
    files += [
        line
        for line in git("ls-files", "--others", "--exclude-standard").splitlines()
        if line
    ]
    return sorted(set(files))


def removed_symbols(base: str) -> dict[str, set[str]]:
    removed: dict[str, set[str]] = {}
    current = None
    for line in git("diff", "-U0", base).splitlines():
        if line.startswith("+++ b/"):
            current = line[6:]
            continue
        if line.startswith("---") or not current:
            continue
        if line.startswith("-"):
            match = DECL_RE.match(line[1:])
            if match:
                removed.setdefault(current, set()).add(match.group(2))
    return removed


def report_changed(base: str, references: dict[str, list[tuple[str, str]]]) -> None:
    files = changed_files(base)
    if not files:
        print("No changed files.")
        return
    removed = removed_symbols(base)
    hits = 0
    print(f"Documentation impact of changes against {base}:")
    for path in files:
        docs = references.get(path)
        if not docs:
            continue
        hits += 1
        by_doc: dict[str, list[str]] = {}
        for doc, symbol in docs:
            by_doc.setdefault(doc, []).append(symbol)
        print(f"\n  {path}")
        for doc, symbols in sorted(by_doc.items()):
            print(f"    {doc}: " + ", ".join(f"`{s}`" for s in sorted(set(symbols))))
        cited = {symbol for _, symbol in docs}
        for symbol in sorted(removed.get(path, set()) & cited):
            print(
                f"    WARNING: the diff removes `{symbol}`, which this doc cites; "
                "update the reference"
            )
    if not hits:
        print("  No documentation references the changed files.")
    else:
        print("\nRe-read the docs above before committing.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--changed", action="store_true")
    parser.add_argument("--base", default="HEAD")
    args = parser.parse_args()

    errors, references = validate()
    if errors:
        for error in errors:
            print(f"ERROR {error}")
        print(f"\nFAIL: {len(errors)} documentation problem(s)")
        return 1
    print(
        f"OK: {len(references)} code file(s) referenced by docs, "
        "all links and cited symbols resolve"
    )
    if args.changed:
        print()
        report_changed(args.base, references)
    return 0


if __name__ == "__main__":
    sys.exit(main())
