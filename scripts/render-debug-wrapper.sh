#!/bin/sh
set -eu

template_path=$1
exec_path=$2
output_path=$3

mkdir -p "$(dirname "$output_path")"
tmp_path="${output_path}.tmp"

sed "s|@APP_EXEC@|$exec_path|g" "$template_path" > "$tmp_path"
chmod 755 "$tmp_path"
mv "$tmp_path" "$output_path"
