#!/usr/bin/env bash
# Watches the honggfuzz overnight campaign log. Invoked by tail-hfuzz.bat.
# Live-follows /tmp/hfuzz-overnight.out while the campaign runs; if it is
# not running, shows the last lines (or a hint if it never ran). Close the
# window (or Ctrl+C) to stop following.

if pgrep -f '[r]un_overnight\.sh|[o]z-hfuzz-target|honggfuzz' >/dev/null 2>&1; then
    echo "── campaign log: /tmp/hfuzz-overnight.out (live; close window to stop) ──"
    tail -f /tmp/hfuzz-overnight.out
elif [ -f /tmp/hfuzz-overnight.out ]; then
    echo "── no campaign running — last lines of /tmp/hfuzz-overnight.out ──"
    tail -n 20 /tmp/hfuzz-overnight.out
else
    echo "[INFO] No campaign log yet - the campaign is not running."
    echo "       Start it with run-hfuzz-overnight.bat first."
fi
