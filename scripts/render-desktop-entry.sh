#!/bin/sh
set -eu

template_path=$1
exec_path=$2
output_path=$3

mkdir -p "$(dirname "$output_path")"
tmp_path="${output_path}.tmp"

while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
        Exec=*)
            printf 'Exec=%s\n' "$exec_path"
            ;;
        TryExec=*)
            printf 'TryExec=%s\n' "$exec_path"
            ;;
        *)
            printf '%s\n' "$line"
            ;;
    esac
done < "$template_path" > "$tmp_path"

mv "$tmp_path" "$output_path"
