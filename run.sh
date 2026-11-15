#!/bin/sh
set -eu
cd -- "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"

if [ -x ./MoonBase101 ]; then
    exec ./MoonBase101 "$@"
fi
case "${1-}" in
    --x11) variant=x11; shift ;;
    --wayland) variant=wayland; shift ;;
    *)
        if [ -f ./MoonBase101-Wayland ]; then
            variant=wayland
        elif [ -f ./MoonBase101-X11 ]; then
            variant=x11
        elif [ -n "${WAYLAND_DISPLAY-}${WAYLAND_SOCKET-}" ]; then
            variant=wayland
        else
            variant=x11
        fi
        ;;
esac

case "$variant" in
    wayland)
        if [ -z "${WAYLAND_DISPLAY-}${WAYLAND_SOCKET-}" ]; then
            printf '%s\n' 'Uruchom tę wersję w sesji Wayland, np. w terminalu Hyprland.' >&2
            exit 1
        fi
        binary=./MoonBase101-Wayland
        ;;
    x11)
        if [ -z "${DISPLAY-}" ]; then
            printf '%s\n' 'Uruchom tę wersję w sesji z serwerem X11 (DISPLAY).' >&2
            exit 1
        fi
        binary=./MoonBase101-X11
        ;;
esac

if [ -f "$binary" ]; then
    if [ ! -x "$binary" ]; then
        chmod u+x "$binary"
    fi
    exec "$binary" "$@"
fi
if [ ! -f Cargo.toml ]; then
    printf 'W tej paczce brakuje pliku %s. Rozpakuj ją ponownie.\n' "$binary" >&2
    exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' 'Brak Cargo. Zainstaluj Rust i zależności opisane w LINUX.md.' >&2
    exit 1
fi
cargo build --release --locked --no-default-features --features "linux-$variant"
exec "${CARGO_TARGET_DIR:-target}/release/moon-base-101" "$@"
