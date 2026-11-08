#!/usr/bin/env bash
# Rust does not build at a usable speed on /mnt/c under WSL, so this workspace is
# built and tested from a native-FS copy. This script is the ONLY way source
# should move between the two:
#
#   push  repo -> build copy, then prove the copy is byte-identical and install
#         each program keypair as target/deploy/<program>-keypair.json
#   pull  build copy -> repo: lockfiles, IDL and TS types, the built client
#         package and its tarball (all generated over there)
#
# A build copy that silently drifts from the repo means testing and deploying
# code that is not the code you are reading.
set -euo pipefail

SRC="$(cd "$(dirname "$0")/.." && pwd)"
DST="${ACME_SOLANA_BUILD_DIR:-$HOME/acme-solana}"
KEYS="${ACME_SOLANA_KEYS:-$HOME/.config/solana/acme}"
# <lib name> <declare_id! file> <keypair file in $KEYS>
PROGRAMS=(
  "arena programs/arena/src/lib.rs arena.json"
  "house programs/house/src/lib.rs house.json"
)
TRACKED=(programs tests scripts client Anchor.toml Cargo.toml rust-toolchain.toml package.json .gitignore)

checksums() { # <dir> <paths...>  (generated client files are not source)
  local dir=$1; shift
  ( cd "$dir" && find "$@" \( -name node_modules -o -name dist -o -path client/src/idl \) -prune \
      -o -type f ! -name '*.tgz' ! -path client/package-lock.json -print0 2>/dev/null | sort -z | xargs -0 sha256sum )
}

case "${1:-}" in
  push)
    mkdir -p "$DST"
    # P(rotect): lockfiles are generated in the build copy, so never delete them
    # there just because the repo does not have them yet.
    # (each file it changes is touched after: a file keeps its time from the repo, which can be older than the last
    # build - another branch pushed and built since - and cargo would then keep that build)
    rsync -a --delete --out-format='%n' \
      --exclude target --exclude node_modules --exclude .anchor \
      --exclude test-ledger --exclude .surfpool --exclude /idl \
      --exclude dist --exclude '*.tsbuildinfo' --exclude '*.tgz' --exclude /client/src/idl \
      --filter='P Cargo.lock' --filter='P package-lock.json' \
      "$SRC/" "$DST/" | while IFS= read -r f; do [[ -f "$DST/$f" ]] && touch "$DST/$f"; done
    paths=()
    for f in "${TRACKED[@]}"; do [[ -e "$SRC/$f" ]] && paths+=("$f"); done
    for f in Cargo.lock package-lock.json client/package-lock.json; do [[ -f "$SRC/$f" ]] && paths+=("$f"); done
    a=$(mktemp); b=$(mktemp); trap 'rm -f "$a" "$b"' EXIT
    checksums "$SRC" "${paths[@]}" > "$a"
    checksums "$DST" "${paths[@]}" > "$b"
    if ! diff -q "$a" "$b" >/dev/null; then
      echo "BUILD COPY DIFFERS FROM REPO:"; diff "$a" "$b"; exit 1
    fi

    # Program ids: the keypairs live outside both trees; anchor build/deploy read them from here.
    mkdir -p "$DST/target/deploy"
    ids=""
    for p in "${PROGRAMS[@]}"; do
      read -r lib src key <<< "$p"
      want=$(grep -oP 'declare_id!\("\K[1-9A-HJ-NP-Za-km-z]+' "$SRC/$src")
      have=$(solana-keygen pubkey "$KEYS/$key")
      if [[ "$want" != "$have" ]]; then
        echo "$lib: declare_id! ($want) != $KEYS/$key ($have)"; exit 1
      fi
      install -m 600 "$KEYS/$key" "$DST/target/deploy/$lib-keypair.json"
      ids+=" $lib=$have"
    done
    echo "build copy $DST == repo $SRC ($(wc -l < "$a") files);$ids"
    ;;
  pull)
    for f in Cargo.lock package-lock.json client/package-lock.json; do
      [[ -f "$DST/$f" ]] && cp "$DST/$f" "$SRC/$f"
    done
    mkdir -p "$SRC/idl"
    for p in "${PROGRAMS[@]}"; do
      read -r lib _ _ <<< "$p"
      cp "$DST/target/idl/$lib.json" "$DST/target/types/$lib.ts" "$SRC/idl/"
    done
    # The built client and its installable tarball (for the site: npm i <tgz>).
    if [[ -d "$DST/client/dist" ]]; then
      rm -rf "$SRC/client/dist" && cp -r "$DST/client/dist" "$SRC/client/dist"
      rm -f "$SRC"/client/*.tgz && cp "$DST"/client/*.tgz "$SRC/client/" 2>/dev/null || true
    fi
    echo "pulled lockfiles + idl/{arena,house}.{json,ts} + client/dist into $SRC"
    ;;
  *)
    echo "usage: $0 push|pull"; exit 2
    ;;
esac
