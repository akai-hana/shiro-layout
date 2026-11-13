#!/bin/sh
# Apply a generated layout to the current X display.
#
# Usage:
#   xkb/install.sh [SYMBOLS_FILE] [NAME]
#   xkb/install.sh [NAME]
#
# With no arguments the shipped layout (xkb/mine) is installed as "mine".
# A single argument may be either a symbols file or a name under xkb/.
#
# setxkbmap cannot be used directly here: its -I flag *replaces* the whole
# system XKB path, so include "us(basic)" inside the symbols file fails to
# resolve ("Error loading new keyboard description"). xkbcomp's -I *adds*
# to the default path, so we let setxkbmap print the component shell and
# compile/send the final keymap with xkbcomp instead.
set -e

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
xkb_dir="${XDG_CONFIG_HOME:-$HOME/.config}/xkb"
display="${DISPLAY:-:0}"

symbols_file="$repo_dir/mine"
name="mine"
if [ "$#" -ge 1 ]; then
    if [ -f "$1" ]; then
        symbols_file="$1"
        name=$(basename -- "$symbols_file")
    else
        name="$1"
        symbols_file="$repo_dir/$name"
    fi
fi
if [ "$#" -ge 2 ]; then
    symbols_file="$1"
    name="$2"
fi

if [ ! -f "$symbols_file" ]; then
    echo "install.sh: symbols file not found: $symbols_file" >&2
    exit 1
fi

mkdir -p "$xkb_dir/symbols"
cp "$symbols_file" "$xkb_dir/symbols/$name"

setxkbmap -layout "$name" -print | xkbcomp "-I$xkb_dir" - "$display"
