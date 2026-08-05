#!/usr/bin/env bash
set -euo pipefail

# Keep this script read-only: it reports missing prerequisites but does not install packages.

failures=0

check_required() {
    local command_name="$1"

    if command -v "${command_name}" >/dev/null 2>&1; then
        printf 'ok       %s\n' "${command_name}"
    else
        printf 'missing  %s\n' "${command_name}"
        failures=$((failures + 1))
    fi
}

check_optional() {
    local command_name="$1"

    if command -v "${command_name}" >/dev/null 2>&1; then
        printf 'ok       %s (optional)\n' "${command_name}"
    else
        printf 'notice   %s is not installed (optional for later features)\n' "${command_name}"
    fi
}

printf 'FurrumX WSL prerequisite check\n'

if grep -Eiq '(microsoft|wsl)' /proc/version 2>/dev/null; then
    printf 'ok       WSL detected\n'
else
    printf 'notice   WSL was not detected; native Linux is also supported\n'
fi

case "${PWD}" in
    /mnt/*)
        printf 'warning  workspace is on a Windows-mounted filesystem\n'
        printf 'warning  use the WSL Linux filesystem for builds and all performance measurements\n'
        ;;
    *)
        printf 'ok       workspace is not under /mnt\n'
        ;;
esac

check_required cargo
check_required rustc
check_required gcc

check_optional clang
check_optional cmake
check_optional pkg-config
check_optional protoc
check_optional python3

if (( failures > 0 )); then
    printf 'failed   %d required prerequisite(s) are missing\n' "${failures}"
    exit 1
fi

printf 'ready    required bootstrap prerequisites are available\n'
