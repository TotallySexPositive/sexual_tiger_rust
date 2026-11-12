#!/bin/sh
set -e

# Ensure directories exist
mkdir -p /usr/src/app/songs /usr/src/app/audio /usr/src/app/images

# Link audio <-> songs directories for legacy volume mount compatibility
# If audio is mounted with files and songs is empty/missing, symlink songs -> audio
if [ -d "/usr/src/app/audio" ] && { [ ! -e "/usr/src/app/songs" ] || [ -z "$(ls -A /usr/src/app/songs 2>/dev/null)" ]; }; then
    rm -rf /usr/src/app/songs
    ln -s /usr/src/app/audio /usr/src/app/songs
elif [ -d "/usr/src/app/songs" ] && { [ ! -e "/usr/src/app/audio" ] || [ -z "$(ls -A /usr/src/app/audio 2>/dev/null)" ]; }; then
    rm -rf /usr/src/app/audio
    ln -s /usr/src/app/songs /usr/src/app/audio
fi

# Auto-update yt-dlp to stay compatible with YouTube API updates
yt-dlp -U || true

# Execute the container command
exec "$@"
