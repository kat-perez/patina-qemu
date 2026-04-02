#!/bin/bash
# Setup CLANGPDB toolchain shims for macOS ARM64 stuart builds.
# Run: source setup_macos_clangpdb.sh
# Re-run after reboot (shims are in /tmp).
#
# Prerequisites: brew install llvm qemu
#                softwareupdate --install-rosetta --agree-to-license
#                cargo install cargo-make

set -e

# Support both bash and zsh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-${(%):-%x}}")" && pwd)"
SHIM_DIR="/tmp/clangpdb-bin"

# ---- Check prerequisites ----
missing=()
[ -x /opt/homebrew/opt/llvm/bin/clang ] || missing+=("llvm (brew install llvm)")
[ -x /opt/homebrew/bin/qemu-system-x86_64 ] || missing+=("qemu (brew install qemu)")
command -v cargo >/dev/null || missing+=("rust (rustup)")
command -v cargo-make >/dev/null || missing+=("cargo-make (cargo install cargo-make)")

if [ ${#missing[@]} -gt 0 ]; then
  echo "Missing dependencies:"
  for dep in "${missing[@]}"; do echo "  - $dep"; done
  return 1 2>/dev/null || exit 1
fi

# ---- Create shim directory ----
rm -rf "$SHIM_DIR"
mkdir -p "$SHIM_DIR"

# ---- LLVM symlinks ----
ln -sf /opt/homebrew/opt/llvm/bin/clang "$SHIM_DIR/clang"
ln -sf /opt/homebrew/opt/llvm/bin/llvm-lib "$SHIM_DIR/llvm-lib"

# ---- llvm-rc wrapper (LLVM 22 path length bug + /Fo flag compat) ----
cat > "$SHIM_DIR/llvm-rc" << 'RCWRAPPER'
#!/bin/bash
REAL_RC=/opt/homebrew/opt/llvm/bin/llvm-rc
REAL_CVTRES=/opt/homebrew/opt/llvm/bin/llvm-cvtres
args=(); output=""; input=""
for arg in "$@"; do
  if [[ "$arg" == /Fo* || "$arg" == /FO* ]]; then output="${arg#/[Ff][Oo]}"
  elif [[ "$arg" == *.rc ]]; then input="$arg"
  else args+=("$arg"); fi
done
[ -z "$input" ] && exec "$REAL_RC" "$@"
tmpdir=$(mktemp -d)
cp "$input" "$tmpdir/input.rc"
if [[ -n "$output" && "$output" == *.lib ]]; then
  "$REAL_RC" "${args[@]}" /FO "$tmpdir/output.res" "$tmpdir/input.rc" || { rm -rf "$tmpdir"; exit 1; }
  "$REAL_CVTRES" /OUT:"$output" "$tmpdir/output.res" || { rm -rf "$tmpdir"; exit 1; }
elif [[ -n "$output" ]]; then
  "$REAL_RC" "${args[@]}" /FO "$tmpdir/output.res" "$tmpdir/input.rc" || { rm -rf "$tmpdir"; exit 1; }
  cp "$tmpdir/output.res" "$output"
else
  "$REAL_RC" "${args[@]}" "$tmpdir/input.rc" || { rm -rf "$tmpdir"; exit 1; }
fi
rm -rf "$tmpdir"
RCWRAPPER
chmod +x "$SHIM_DIR/llvm-rc"

# ---- lld-link from Rust toolchain ----
DXE_CORE_REPO="$SCRIPT_DIR/patina-dxe-core-qemu"
if [ -f "$DXE_CORE_REPO/rust-toolchain.toml" ]; then
  TOOLCHAIN=$(grep 'channel' "$DXE_CORE_REPO/rust-toolchain.toml" | sed 's/.*"\(.*\)".*/\1/' | head -1)
  rustup toolchain install "$TOOLCHAIN" --profile minimal 2>/dev/null || true
  SYSROOT=$(rustc +"$TOOLCHAIN" --print sysroot 2>/dev/null)
  if [ -n "$SYSROOT" ]; then
    ln -sf "$SYSROOT/lib/rustlib/aarch64-apple-darwin/bin/rust-lld" "$SHIM_DIR/lld-link"
    ln -sf "$SYSROOT/lib/rustlib/aarch64-apple-darwin/bin/rust-lld" "$SHIM_DIR/rust-lld"
  else
    echo "WARNING: Could not find Rust sysroot for $TOOLCHAIN"
  fi
else
  echo "WARNING: $DXE_CORE_REPO/rust-toolchain.toml not found"
  echo "  Clone patina-dxe-core-qemu first, then re-run this script."
fi

# ---- Compile BaseTools C binaries (needed for GenFfs, GenFv, etc.) ----
BASETOOLS_C="$QEMU_REPO/MU_BASECORE/BaseTools/Source/C"
if [ -d "$BASETOOLS_C" ] && [ ! -f "$BASETOOLS_C/bin/GenFfs" ]; then
  echo "Compiling BaseTools C binaries..."
  make -C "$BASETOOLS_C" > /dev/null 2>&1 && echo "  Done." || echo "  WARNING: BaseTools C compilation failed"
fi

# ---- Set environment variables ----
QEMU_REPO="$SCRIPT_DIR/patina-qemu"
export CLANG_BIN="$SHIM_DIR/"
export NASM_PREFIX="$QEMU_REPO/MU_BASECORE/BaseTools/Bin/mu_nasm_extdep/MacOs-x86-64/"
export PATH="$NASM_PREFIX:$QEMU_REPO/MU_BASECORE/BaseTools/BinWrappers/PosixLike:/opt/homebrew/bin:$PATH"

# ---- Verify ----
echo ""
echo "CLANGPDB macOS ARM64 setup complete:"
for tool in clang llvm-lib llvm-rc lld-link rust-lld; do
  if [ -e "$SHIM_DIR/$tool" ]; then
    echo "  OK  $tool"
  else
    echo "  MISSING  $tool"
  fi
done
echo ""
echo "Environment set:"
echo "  CLANG_BIN=$CLANG_BIN"
echo "  NASM_PREFIX=$NASM_PREFIX"
echo ""
echo "Ready to build. From patina-qemu/:"
echo "  stuart_build -c Platforms/QemuQ35Pkg/PlatformBuild.py TOOL_CHAIN_TAG=CLANGPDB"
