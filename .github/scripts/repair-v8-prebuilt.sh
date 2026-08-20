#!/usr/bin/env bash
# v8-goose caches its prebuilt librusty_v8.a behind a small librusty_v8.sum
# marker holding the download URL, and its build script returns early whenever
# that marker matches - without checking the archive is still present. Both
# files live under target/, which the Rust cache action prunes before saving, so
# a restored cache can keep the marker and drop the archive. The build script
# then skips the download and linking fails with "could not find native static
# library `rusty_v8`". Drop any marker whose archive is missing so the download
# runs again.
set -euo pipefail

[ -d target ] || exit 0

find target -type f -path '*gn_out/obj*' -name '*.sum' -print0 |
  while IFS= read -r -d '' marker; do
    for archive in "${marker%.sum}.a" "${marker%.sum}.lib"; do
      [ -f "$archive" ] && continue 2
    done
    echo "Removing stale v8 prebuilt marker without its archive: $marker"
    rm -f "$marker"
  done
