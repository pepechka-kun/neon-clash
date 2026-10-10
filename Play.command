#!/bin/zsh
set -e
GAME_ROOT="${0:A:h}"
cd "$GAME_ROOT"
exec "$GAME_ROOT/dist/neon-clash" "$@"
