# OpenSeq — put a project-local Rust toolchain on PATH for this shell.
#
#   source scripts/rust-env.sh
#   cd crates/core && cargo test          # 25 unit + integration tests
#   cargo build -p openseq                # the desktop app
#
# Why this script exists at all
# ----------------------------
# OpenSeq was developed on a machine with no MSVC and no system-wide Rust, using
# an `x86_64-pc-windows-gnu` toolchain plus a MinGW-w64 prefix that both live
# **next to** the repository rather than in the system. If you have a normal
# Rust install, ignore this script and just run `cargo`.
#
# It expects this layout (override with the env vars below if yours differs):
#
#   <workspace>/
#   ├── .rust/                     rustc + cargo (the toolchain)
#   ├── .mingw/extract/mingw64/    binutils + gcc: as ld dlltool windres gcc
#   └── logseq-tauri/              this repository
#
# Why the MinGW tools are needed: Rust's "self-contained" GNU toolchain ships
# dlltool and ld but **no assembler**, and crates that use `raw-dylib` (the
# `windows` crate family) make rustc synthesise import libraries at build time —
# dlltool does that by shelling out to `as`. tauri-build additionally uses
# `windres`, which preprocesses the .rc with a real `gcc -E`.
#
# Env overrides: OPENSEQ_WORKSPACE (workspace root in Windows form, e.g.
# E:/git), OPENSEQ_MINGW_ROOT (the `mingw64` prefix itself).
#
# Note: this file is meant to be *sourced*, so it must not rely on `$0` — that
# is the calling shell, not this script. BASH_SOURCE is what we need.

_self="${BASH_SOURCE[0]:-$0}"
_here="$(cd "$(dirname "${_self}")" && pwd)"

if [ -n "${OPENSEQ_WORKSPACE:-}" ]; then
  WORKSPACE_WIN="${OPENSEQ_WORKSPACE}"
  WORKSPACE_POSIX="/$(echo "${WORKSPACE_WIN}" | sed -E 's|^([A-Za-z]):|\L\1|')"
else
  # <repo>/scripts/../.. == the workspace that holds .rust and .mingw
  WORKSPACE_POSIX="$(cd "${_here}/../.." && pwd)"
  WORKSPACE_WIN="$(echo "${WORKSPACE_POSIX}" | sed -E 's|^/([A-Za-z])/|\U\1:/|')"
fi

# PATH entries must be in POSIX form for Git Bash; cargo/rustup want the Windows
# form for their own home directories.
RUST_HOME_WIN="${WORKSPACE_WIN}/.rust"
RUST_HOME_POSIX="${WORKSPACE_POSIX}/.rust"
MINGW_PREFIX="${OPENSEQ_MINGW_ROOT:-${WORKSPACE_POSIX}/.mingw/extract/mingw64}"

export CARGO_HOME="${RUST_HOME_WIN}/cargo"
export RUSTUP_HOME="${RUST_HOME_WIN}/rustup"
export PATH="${MINGW_PREFIX}/bin:${RUST_HOME_POSIX}/cargo/bin:${PATH}"

# Use Rust's bundled ld and import libraries, but take as/ld/dlltool/windres/gcc
# from the MinGW prefix on PATH.
export RUSTFLAGS="-C link-self-contained=yes"

echo "workspace: ${WORKSPACE_WIN}"
command -v cargo >/dev/null 2>&1 && cargo --version || echo "cargo not found — set OPENSEQ_WORKSPACE, or install Rust normally"
