#!/usr/bin/env bash
# Builds the Windows installer (NSIS) from Linux/WSL.
# Needs: cargo-xwin, pnpm, and the llvm, clang, lld and nsis packages.
# Output: target/x86_64-pc-windows-msvc/release/bundle/nsis/
set -euo pipefail
cd "$(dirname "$0")/.."

target=x86_64-pc-windows-msvc

# The app embeds the extension, and the installer ships the service and CLI.
(cd extension && pnpm install --frozen-lockfile && pnpm build)
cargo xwin build --release --target "$target" -p focuslock-service -p focuslock-cli

cd apps/desktop
pnpm install --frozen-lockfile
pnpm tauri build --runner cargo-xwin --target "$target"
