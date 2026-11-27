#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 OSTREE_REPOSITORY OUTPUT.flatpak" >&2
    exit 2
fi

remote="$(dirname "$0")/linux-ds.flatpakrepo"
repo_url=$(sed -n 's/^Url=//p' "$remote")
public_key=$(sed -n 's/^GPGKey=//p' "$remote")
if [ -z "$repo_url" ] || [ -z "$public_key" ]; then
    echo "Missing repository URL or public key in $remote" >&2
    exit 1
fi

key_file=$(mktemp)
trap 'rm -f "$key_file"' EXIT
printf '%s' "$public_key" | base64 --decode > "$key_file"

flatpak build-bundle --arch=x86_64 \
    --repo-url="$repo_url" \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    --gpg-keys="$key_file" \
    "$1" "$2" org.linuxds.Schnapsen master
