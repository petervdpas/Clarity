#!/usr/bin/env bash
# Installs a Clarity package in a clean container of the distribution it is
# for, as a person would, and checks what can be checked without a screen:
# the package manager finds every dependency, the program finds every
# library it links, and the menu entry is there.
#
#   install-test.sh <package file>
set -euo pipefail

package="$1"

case "$package" in
    *.deb)
        export DEBIAN_FRONTEND=noninteractive
        apt-get update
        apt-get install -y "./$package"
        ;;
    *.rpm)
        dnf -y install "./$package"
        ;;
    *)
        echo "not a package: $package" >&2
        exit 1
        ;;
esac

program=/usr/bin/clarity-app
test -x "$program" || { echo "$program is not installed" >&2; exit 1; }
missing="$(ldd "$program" | grep "not found" || true)"
if [ -n "$missing" ]; then
    echo "libraries the program cannot find:" >&2
    echo "$missing" >&2
    exit 1
fi
ls /usr/share/applications/ | grep -i clarity
echo "installed: $package"
