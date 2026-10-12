# Architecture notes

## The one invariant

> The Markdown files are the source of truth. Everything in memory is a cache
> derived from them, and it can be thrown away and rebuilt at any time.

That is what makes "local-first" more than a slogan here: there is no database,
no migration, no hidden state. Delete the app and your notes are still a folder
of `.md` files that Logseq itself can open.

## Layering

```
┌──────────────────────────────────────────────────────────────┐
│ React + TypeScript  (src/)                                   │
│   components  →  store (zustand)  →  api.ts  →  invoke       │
└───────────────────────────┬──────────────────────────────────┘
                            │ Tauri IPC (JSON)
┌───────────────────────────┴──────────────────────────────────┐
│ Tauri shell  (src-tauri/)                                    │
│   commands.rs   state.rs   watcher.rs                        │
└───────────────────────────┬──────────────────────────────────┘
                            │ plain Rust calls
┌───────────────────────────┴──────────────────────────────────┐
│ logseq-core  (crates/core/)                                  │
│   parser  serializer  store  refs  search  model             │
└──────────────────────────────────────────────────────────────┘
```

The split is deliberate: **`crates/core` has no Tauri dependency at all**, so
all of the interesting logic is covered by `cargo test`. The desktop shell is
adapter code with almost no branching.

## Who owns what

| Concern | Owner | Why |
| --- | --- | --- |
| Block-tree edits (indent, merge, reorder) | TypeScript (`lib/outline.ts`) | They are driven by caret/key events; a round-trip per keystroke would be silly. |
| Markdown ⇄ block tree | Rust (`parser.rs` / `serializer.rs`) | One implementation, one set of tests, no drift between two languages. |
| Persistence, indexing, backlinks | Rust (`store.rs`) | Needs to touch the filesystem anyway; keeping it near the files avoids stale caches. |
| Rendering inline syntax | TypeScript (`lib/inline.tsx`) | Purely presentational. |

Because the frontend hands the **whole page** back on every save, and Rust
re-serialises it, there is no partial-write state to reconcile.

## The Markdown dialect

Everything the parser understands is in `parser.rs`:

```text
title:: My Page              page property — no bullet, indent 0
tags:: a, b

- A top level block
  - A child block
    prop:: value             block property — indented under its parent
  - TODO [#A] a task
  - ```js                    fenced code: following lines are content
    const a = 1
    ```
  continued line of the parent  ← continuation, no bullet
```

Rules:

- Indentation defines nesting; two spaces per level, a tab counts as two.
- A non-bullet line more indented than the current block is folded into its
  `content` (this is how multi-line blocks survive a round trip).
- A non-bullet `key:: value` line becomes a property of the enclosing block,
  or of the page when it sits at indent 0.
- Blank lines are separators and are normalised away. **This is a deliberate
  fidelity trade-off:** Logseq rewrites files on save, and so does OpenSeq.

`tests/roundtrip.rs::parse_serialize_is_a_fixed_point` asserts
`serialize(parse(x)) == serialize(parse(serialize(parse(x))))`, i.e. the
serialiser is idempotent — the property that actually matters for a file
format that users edit both inside and outside the app.

## Block identity

Logseq only writes a block id to disk when the block is referenced. OpenSeq
mirrors that:

- every block gets a fresh v4-shaped id in memory, produced by `src/id.rs`
  (process-local seed + monotonic counter, not a CSPRNG — see *Dependencies*);
- `id_in_file` is `false` until the user asks for a stable reference
  (`((block ref))`), which the frontend triggers via `ensure_block_id`;
- when `id_in_file` is true, the serialiser emits `id:: <uuid>` as a property
  line, and the parser folds it back into `Block::id`.

Consequence: block ids are **not** stable across restarts unless stamped.
Documented, not hidden.

## Search

`search.rs` builds an inverted index from the in-memory page map after every
save (`reindex`). Terms:

- ASCII: runs of `[A-Za-z0-9_]`, lower-cased;
- CJK: each character, **plus** each overlapping bigram.

Bigrams are the cheapest thing that behaves acceptably for Chinese without a
segmentation dictionary: `知识管理` indexes `知 知识 识 识管 管 管理 理`, so
`知识` and `管理` both hit. Queries are tokenised identically; scoring is
token-overlap plus an exact-substring bonus, with a linear-scan fallback for
partial queries.

Rebuilding instead of patching the index is O(blocks) per save. For a personal
graph that is well under a frame budget, and it makes "the index is stale"
impossible — a class of bug worth more than the cycles.

## Dependencies

`crates/core` depends on exactly four crates: `serde`, `serde_json`, `regex`
and `thiserror`. That is a deliberate constraint, not an accident:

- **No C code, no `raw-dylib`.** Crates that link Windows DLLs through
  `raw-dylib` (`getrandom`, `windows-sys` ≥ 0.59) force rustc to synthesise
  import libraries at build time, which needs `dlltool` + an assembler. On a
  machine with no MSVC and a self-contained GNU toolchain, that fails outright.
- **A bare toolchain is enough.** With the dependency set above, `cargo test`
  works with nothing but `rustc`/`cargo` — no extra flags, no PATH surgery, no
  `-C link-self-contained`.
- **Three conveniences were dropped to get there**, each replaced by something
  small and local: `walkdir` → a 20-line depth-limited directory walk
  (`store.rs`); `uuid` → `id.rs`; `tempfile` (dev-only) → a `TempDir` helper in
  the test file.

Block ids are the one place where this touches behaviour: a CSPRNG would be
stricter, but a block id only has to be unique *inside one user's graph* — there
is no adversary choosing inputs. The generator still emits a 36-character,
version-4-shaped uuid, so `id::` values stay interoperable with Logseq.
Swapping it back to `uuid`'s `v4` feature is a one-line change if you ever want
cryptographic randomness.

## Building the desktop shell on Windows

`crates/core` builds with nothing but `rustc`/`cargo`. The **shell** (`src-tauri`)
is a different story, because Tauri pulls in the `windows` crate family:

- those crates link via `raw-dylib`, so rustc synthesises import libraries at
  build time and needs **`dlltool` plus an assembler (`as`)**;
- `tauri-build` embeds the app icon with **`windres`**, and windres preprocesses
  the `.rc` with **`gcc -E`** — so a real gcc (with `cc1`) is required, not just
  binutils.

Rust's self-contained GNU toolchain ships `dlltool`/`ld` but neither `as` nor a
working `gcc`. So on a machine without MSVC you need a MinGW-w64 prefix on PATH —
`scripts/rust-env.sh` points at the one installed under the workspace
(`E:/git/.mingw`) and enables `-C link-self-contained=yes` to keep using Rust's
bundled `ld` and import libraries.

Two more shell-specific notes:

- `[lib] crate-type` is `["lib"]`, not the template's
  `["staticlib", "cdylib", "rlib"]`. The `cdylib` is for mobile; on Windows GNU
  it makes `ld` emit an export table for the whole dependency graph and fail with
  `export ordinal too large`. Add it back only when building for mobile.
- `src-tauri` carries one unused build-dependency (`indexmap` with the `std`
  feature). It exists to fix `schemars 0.8` (used by `tauri-build`) failing with
  `E0107` — see the comment in `src-tauri/Cargo.toml`.

## Concurrency

All state lives behind one `Mutex<AppState>` and commands are synchronous.
Pages are small, so there is no benefit in async file IO; the lock is never held
across an `await`. Writes go through `write_atomic` (temp file + rename) so a
crash mid-save leaves the previous version intact.

## Where to extend

- **Query DSL** — add a `query.rs` that compiles `{{query …}}` to a filter over
  `Graph::pages()`; the graph is already in memory.
- **Org-mode** — `parser.rs` is line-oriented; a second dialect is a sibling
  module, not a rewrite.
- **Plugin-ish extensions** — the command layer is the natural seam: expose a
  stable set of commands and let extensions call them.
- **Sync** — because files are the source of truth, sync is "make the folder
  converge", not "merge two databases".
