@echo off
REM OpenSeq — put the workspace-local Rust toolchain on PATH for this session.
REM Usage:  scripts\rust-env.cmd  &&  cargo test -p logseq-core
REM
REM Everything lives inside the workspace on purpose — nothing machine-wide:
REM   E:\git\.rust   Rust toolchain (rustc/cargo)
REM   E:\git\.mingw  MinGW binutils + gcc (as, ld, dlltool, windres, gcc)
REM
REM Why the MinGW tools are needed: Rust's "self-contained" GNU toolchain has
REM dlltool and ld but no assembler (raw-dylib needs `as`), and tauri-build's
REM windres needs a real `gcc` for preprocessing the .rc file.

set "RUST_ROOT=%~dp0..\..\.rust"
set "MINGW_ROOT=%~dp0..\..\.mingw\extract\mingw64"

set "CARGO_HOME=%RUST_ROOT%\cargo"
set "RUSTUP_HOME=%RUST_ROOT%\rustup"
set "PATH=%MINGW_ROOT%\bin;%CARGO_HOME%\bin;%PATH%"
set "RUSTFLAGS=-C link-self-contained=yes"

cargo --version
