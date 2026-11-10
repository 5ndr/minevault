#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
if [ -d "$DIR/macos/ResticDashboard.app" ]; then
    open "$DIR/macos/ResticDashboard.app"
elif [ -f "$DIR/macos/resticdashboard" ]; then
    "$DIR/macos/resticdashboard"
else
    echo "ResticDashboard executable not found in $DIR/macos"
    read -p "Press Enter to exit..."
fi
