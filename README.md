# `example-cargo-workspace` — sibling sources in a cargo workspace

<p align="center">
  <img src="docs/assets/jobs-logo.jpg" alt="JOBS — Jonas' Own Build System" width="520">
</p>

A worked example of **sibling sources** (JOBS sibling-sources design,
2026-07-26) for Rust: a `BUILD.jobs` in a workspace member whose crate depends
on a **sibling crate** through a plain cargo `path = "../greeting"` dependency
— the thing that was inexpressible before context widening. The cargo plugin
discovers the path-dep closure from the workspace, the engine covers exactly
that closure, and two **generated files** — a reduced root `Cargo.toml` and a
pruned root `Cargo.lock` — make the covered subtree a coherent little
workspace of its own.

## What's here

```
example-cargo-workspace/
├── README.md
├── Cargo.toml              ← [workspace] members = app, greeting, unused; [workspace.dependencies] heck
├── Cargo.lock              ← REAL lock for the whole workspace (heck included)
├── docs/
│   └── notes.md            ← deliberately unrelated — the memo demo (see below)
└── crates/
    ├── greeting/           ← lib crate, no external deps; the sibling app pulls in
    ├── unused/             ← lib crate using heck — NOT in app's closure, exists to be pruned
    └── app/                ← bin crate, `greeting = { path = "../greeting" }`
        ├── BUILD.jobs      ← the recipe: cargo plugin in workspace mode + offline musl build
        ├── Cargo.toml
        └── src/main.rs
```

The workspace's only external dependency (`heck`, dependency-free) is used by
`crates/unused` **only** — so the whole point of lockfile pruning is
observable: it is in the committed `Cargo.lock`, and it is absent from the
lock that `app`'s build is keyed on.

## How it works

1. **Context widening.** From a checkout, `--source crates/app` defaults the
   ingest root to the **git repo root** (`.git` itself is never ingested), so
   the whole workspace is the build's context and `dir = crates/app`.
2. **`plugins()`** declares the cargo plugin (pinned tarball of
   [`jobs-build/plugin-cargo`](https://github.com/jobs-build/plugin-cargo)).
3. **`build()`** calls it in **workspace mode** — `plugins["cargo"](workspace
   = True)` — and the plugin reads the mounted context itself: it finds the
   workspace root above `crates/app`, walks the crate's `path`-dependency
   closure transitively (including `dep = { workspace = true }` entries whose
   `[workspace.dependencies]` definition carries a `path`), and answers
   `{crates, sources, generated}`:
   - `sources = ["//crates/greeting"]` — the covered siblings;
   - `generated["//Cargo.toml"]` — a **reduced** root manifest whose
     `members` list only the closure (`crates/app`, `crates/greeting`);
   - `generated["//Cargo.lock"]` — a **pruned** lock holding only the
     `[[package]]` entries reachable from the closure through the lock's own
     dependency edges (a resolution-aware graph walk, not a text filter) —
     `heck` and `unused` are gone;
   - `crates` — one crates.io import per entry of the *pruned* lock (zero,
     for `app`), so only the closure's registry deps are ever fetched.
4. The recipe forwards `resp["sources"]` into `sources =` and
   `resp["generated"]` into `generated =` on the `build()` return. The
   engine's covered closure is then `crates/app` + the recipe +
   `crates/greeting`, with the generated pair overlaid at the workspace root
   — and that tree, nothing else, keys the build (KP).
5. The build sandbox materializes the covered tree at `$SRC_ROOT`
   (`/build/src`) with CWD `$SRC = /build/src/crates/app`: cargo walks up
   from the crate, finds the reduced workspace root and pruned lock at
   `$SRC_ROOT`, and `cargo build --release --frozen` succeeds offline — the
   pruned lock exactly matches the reduced workspace's resolution. The rest
   is the [`rust-build`
   example](https://github.com/jobs-build/examples/tree/main/rust-build)'s
   offline musl toolchain: pinned `rust-1.96.0` musl dist, vendored registry
   config, Rust's bundled `rust-lld`, no C toolchain.

## Run it

From a checkout of this repo (Linux, `jobs-client` from
[jobs-iroh](https://github.com/fables-for-robots/jobs-iroh)):

```bash
jobs-client build --source crates/app    # hermetic offline build → …/bin/app
jobs-client run   --source crates/app    # build, then execute → "hello from crates/greeting"
```

The git-root default widens the context automatically — no flags needed.
(`--source-root` overrides the root; `--no-repo-root` disables the widening.)

## The memo demo (early cutoff at closure granularity)

The build is keyed by **KP** — the content of the covered closure
(`crates/app` + `crates/greeting` + the generated manifest/lock), not the
whole repo:

```bash
echo "- meeting notes" >> docs/notes.md
jobs-client build --source crates/app
#   ✓ build example-cargo-workspace app  (cached)   ← outside the closure: memo hit

echo '// tweak' >> crates/unused/src/lib.rs        # any edit to the pruned member
jobs-client build --source crates/app
#   ✓ build example-cargo-workspace app  (cached)   ← STILL cached: unused is pruned
#     out of both the members list and the lock, so its content never enters KP

sed -i 's/hello from/hi from/' crates/greeting/src/lib.rs
jobs-client run --source crates/app
#   rebuilds — the sibling IS covered — and prints the new greeting
```

The middle case is the headline: `crates/unused` is a real workspace member
sharing the real root `Cargo.toml` and `Cargo.lock` with `app` — in any
whole-tree-keyed system, editing it (or bumping its `heck` dependency, which
rewrites the shared lock) would rebuild `app`. Here the plugin regenerates
the reduced manifest and pruned lock, their bytes come out identical, KP is
unchanged, and the build memo-hits. Timestamps don't matter either — the
covered tree is normalized before hashing, so `touch` and fresh checkouts of
the same bytes land on the same KP.

## Notes

- The pruning is **resolution-aware, never textual**: cargo unifies versions
  across the whole workspace, so an unrelated member's bump can legitimately
  change *your* resolved versions — the lock-graph slice catches exactly
  that. If `app` ever gains a dep that unifies with one of `unused`'s, the
  pruned lock changes and the rebuild happens, correctly.
- `[workspace.dependencies]` passes through the reduced manifest **verbatim**
  (it is semantically load-bearing for the surviving members); an entry no
  member inherits — like `heck` here — is inert for resolution, which is why
  the pruned lock can drop it while the table still names it.
- The committed `Cargo.lock` is a real `cargo generate-lockfile` product for
  the **whole** workspace; nothing about the repo is JOBS-specific except
  `crates/app/BUILD.jobs`.
- `cargo build`/`cargo run` from the checkout work exactly as in any cargo
  workspace — JOBS reads the repo, it never rewrites it.
