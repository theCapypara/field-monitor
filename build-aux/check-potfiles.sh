#!/usr/bin/env bash
POTFILES="po/POTFILES.in"

potfile-lines() {
    # Extract non-empty, non-comment lines
    grep -v '^\s*#' "$POTFILES" | grep -v '^\s*$'
}

check-missing-translations() {
    local missing=0

    # Check all .blp files in project source dirs are listed
    for f in $(find src/ lib/ connection/ -name '*.blp' | sort); do
        if ! grep -qxF "$f" "$POTFILES"; then
            echo "::error::Missing from $POTFILES: $f"
            missing=1
        fi
    done

    # Check all .rs files containing "gettext" are listed
    for f in $(grep -rl 'gettext' --include='*.rs' src/ lib/ connection/ | sort); do
        if ! grep -qxF "$f" "$POTFILES"; then
            echo "::error::Missing from $POTFILES: $f"
            missing=1
        fi
    done

    if [ "$missing" -eq 1 ]; then
        echo ""
        echo "Add the missing files to $POTFILES and keep it sorted alphabetically."
        return 1
    fi

    echo "$POTFILES: all translatable sources listed."
}

check-missing-sources() {
    local missing=0

    for line in $(potfile-lines); do
        if ! [ -f "$line" ]; then
            echo "::error::Non-existent file in $POTFILES: $line"
            missing=1
        fi
    done

    if [ "$missing" -eq 1 ]; then
        echo ""
        echo "Remove the missing files from $POTFILES."
        return 1
    fi

    echo "$POTFILES: all source files exist."
}

check-sorting() {
    potfile-lines > /tmp/potfiles_entries
    LC_ALL=C sort /tmp/potfiles_entries > /tmp/potfiles_sorted
    if ! diff -u /tmp/potfiles_entries /tmp/potfiles_sorted; then
        echo "::error::$POTFILES is not sorted alphabetically. Please sort it."
        return 1
    fi
    echo "$POTFILES: sorted correctly."
}

check-missing-translations || exit 1
check-missing-sources || exit 1
check-sorting || exit 1
