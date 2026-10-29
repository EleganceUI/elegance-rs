#!/usr/bin/env bash
# Checks that the shim compiles against an Elegance checkout's headers: a minute, with no
# dependencies and no build. Elegance's public headers need only its CMake-generated
# <elg/export.h>, which a stub stands in for. GNU mode, as the native library builds the shim.
#
#   .github/scripts/check-shim.sh <elegance checkout> [<shim folder>]
#
# CXX picks the compiler (default g++). Prints each file's errors in its own log group.
set -uo pipefail

elegance=$1
shim=${2:-elegance-sys/shim}

generated=$(mktemp -d)
trap 'rm -rf "$generated"' EXIT
mkdir -p "$generated/elg"
cat > "$generated/elg/export.h" <<'EOF'
#ifndef ELG_API_H
#define ELG_API_H
#define ELG_API
#define ELG_NO_EXPORT
#define ELG_DEPRECATED __attribute__((__deprecated__))
#define ELG_DEPRECATED_EXPORT ELG_DEPRECATED
#define ELG_DEPRECATED_NO_EXPORT ELG_DEPRECATED
#define ELG_PRIVATE_MEMBERS_BEGIN
#define ELG_PRIVATE_MEMBERS_END
#endif
EOF

status=0
for file in "$shim"/*.cpp; do
    echo "::group::$file"
    "${CXX:-g++}" -std=gnu++23 -fsyntax-only -DELG_SHIM_SHARED -I"$elegance/include" -I"$generated" "$file" || status=1
    echo "::endgroup::"
done
exit "$status"
