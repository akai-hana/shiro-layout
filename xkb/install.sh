#!/bin/sh
# Apply the cfgE layout (xkb/mine) to the current X display.
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

mkdir -p "$xkb_dir/symbols"
cp "$repo_dir/mine" "$xkb_dir/symbols/mine"

setxkbmap -layout mine -print | xkbcomp "-I$xkb_dir" - "$display"
