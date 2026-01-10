#!/bin/bash
# Prevent native-tls/OpenSSL from being added to the dependency tree.
# These cause Linux compatibility issues with OpenSSL version mismatches.
# See: https://github.com/aaif/goose/issues/6034

set -e

BANNED_CRATES=("native-tls" "openssl-sys" "openssl")
FOUND_BANNED=0

for crate in "${BANNED_CRATES[@]}"; do
    if cargo tree -i "$crate" 2>/dev/null | grep -q "$crate"; then
        echo "ERROR: Found banned crate '$crate' in dependency tree"
        echo "This causes Linux compatibility issues with OpenSSL versions."
        echo "Use rustls-based alternatives instead (e.g., rustls-tls-native-roots)."
        echo ""
        echo "Dependency chain:"
        cargo tree -i "$crate"
        echo ""
        FOUND_BANNED=1
    fi
done

if [ $FOUND_BANNED -eq 1 ]; then
    exit 1
fi

echo "✓ No banned TLS crates found (native-tls, openssl, openssl-sys)"
