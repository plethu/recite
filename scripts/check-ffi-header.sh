#!/usr/bin/env bash
set -euo pipefail

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

header="$repo_root/include/recite.h"
c_probe="$repo_root/tests/ffi-header/aliases.c"
cpp_probe="$repo_root/tests/ffi-header/aliases.cpp"
null_probe="$repo_root/tests/ffi-header/null_callbacks.c"
lifetime_probe="$repo_root/tests/ffi-header/locale_lifetime.c"
lifetime_source="$repo_root/tests/ffi-header/locale_lifetime.recite"
for path in "$header" "$c_probe" "$cpp_probe" "$null_probe" "$lifetime_probe" "$lifetime_source"; do
  [[ -f "$path" ]] || { echo "missing FFI header probe input: $path" >&2; exit 2; }
done

cc_bin="${CC:-cc}"
cxx_bin="${CXX:-c++}"
command -v "$cc_bin" >/dev/null 2>&1 || { echo "missing C compiler: $cc_bin" >&2; exit 2; }
command -v "$cxx_bin" >/dev/null 2>&1 || { echo "missing C++ compiler: $cxx_bin" >&2; exit 2; }

tmpdir="$(mktemp -d /tmp/recite-ffi-header.XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT

"$cc_bin" -std=c11 -Wall -Wextra -Werror -pedantic -I"$repo_root/include" \
  -c "$c_probe" -o "$tmpdir/aliases-c.o"
"$cxx_bin" -std=c++17 -Wall -Wextra -Werror -pedantic -I"$repo_root/include" \
  -c "$cpp_probe" -o "$tmpdir/aliases-cpp.o"

ffi_library=""
for candidate in \
  "$repo_root/target/debug/librecite_ffi.so" \
  "$repo_root/target/debug/librecite_ffi.dylib" \
  "$repo_root/target/debug/recite_ffi.dll"; do
  if [[ -f "$candidate" ]]; then
    ffi_library="$candidate"
    break
  fi
done
if [[ -z "$ffi_library" ]]; then
  cargo build -p recite-ffi --quiet
  for candidate in \
    "$repo_root/target/debug/librecite_ffi.so" \
    "$repo_root/target/debug/librecite_ffi.dylib" \
    "$repo_root/target/debug/recite_ffi.dll"; do
    if [[ -f "$candidate" ]]; then
      ffi_library="$candidate"
      break
    fi
  done
fi
if [[ -z "$ffi_library" ]]; then
  echo "unable to locate the built recite-ffi library for the NULL callback probe" >&2
  exit 2
fi

case "$ffi_library" in
  *.so) library_dir="$(dirname "$ffi_library")"; library_name="recite_ffi" ;;
  *.dylib) library_dir="$(dirname "$ffi_library")"; library_name="recite_ffi" ;;
  *.dll) library_dir="$(dirname "$ffi_library")"; library_name="recite_ffi" ;;
esac
"$cc_bin" -std=c11 -Wall -Wextra -Werror -pedantic -I"$repo_root/include" \
  "$null_probe" -L"$library_dir" -l"$library_name" \
  -Wl,-rpath,"$library_dir" -o "$tmpdir/null-callbacks"
LD_LIBRARY_PATH="$library_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  "$tmpdir/null-callbacks"

"$cc_bin" -std=c11 -Wall -Wextra -Werror -pedantic -I"$repo_root/include" \
  -I"$repo_root/tests/ffi-header" "$lifetime_probe" -L"$library_dir" \
  -l"$library_name" -Wl,-rpath,"$library_dir" -o "$tmpdir/locale-lifetime"
cargo run --quiet --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  compile --output "$tmpdir/locale-lifetime.recitec" "$lifetime_source" >/dev/null
LD_LIBRARY_PATH="$library_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  "$tmpdir/locale-lifetime" "$tmpdir/locale-lifetime.recitec" "$lifetime_source"

echo "FFI header C/C++ alias, linked NULL callback, and locale lifetime probes passed."
