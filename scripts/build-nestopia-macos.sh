#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
REV=60a325237ce3df3a42f8f58cec5ebf0d6be412eb
if [ ! -f vendor/nestopia/libretro/Makefile ]; then
  git clone https://github.com/libretro/nestopia.git vendor/nestopia
fi
# Refuse to discard local changes in third-party sources.
if [ -d vendor/nestopia/.git ]; then
[ -z "$(git -C vendor/nestopia status --porcelain --untracked-files=no)" ] || { echo 'Nestopia has local source edits'; exit 1; }
git -C vendor/nestopia checkout "$REV"
fi
# Match upstream's exported retro_* ABI, with size optimization and dead stripping.
FRAGMENT=$(mktemp)
EXPORTS=$(mktemp)
trap 'rm -f "$FRAGMENT" "$EXPORTS"' EXIT
printf '_retro_*\n' > "$EXPORTS"
cat > "$FRAGMENT" <<EOF
override CFLAGS += -Oz -flto
override CXXFLAGS += -Oz -flto
override LDFLAGS += -flto -Wl,-dead_strip -Wl,-exported_symbols_list,$EXPORTS
EOF
make -C vendor/nestopia/libretro clean >/dev/null
make -C vendor/nestopia/libretro -f Makefile -f "$FRAGMENT" -j8 platform=osx MINVERSION=-mmacosx-version-min=11.0 ARCHFLAGS='-arch arm64'
strip -x vendor/nestopia/libretro/nestopia_libretro.dylib
codesign --force --sign - vendor/nestopia/libretro/nestopia_libretro.dylib
