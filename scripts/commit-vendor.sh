#!/bin/sh
set -eu

if [ $# -eq 0 ]; then
  set -- $(git ls-files --others --exclude-standard vendor | cut -d/ -f2 | sort -u)
fi

if [ $# -eq 0 ]; then
  echo "no untracked vendor crates found"
  exit 0
fi

for crate in "$@"; do
  case "$crate" in
    vendor/*) ;;
    *) crate="vendor/$crate" ;;
  esac

  if [ ! -d "$crate" ]; then
    echo "error: missing directory: $crate" >&2
    exit 1
  fi

  name=${crate##*/}

  git add "$crate"

  if git diff --cached --quiet -- "$crate"; then
    echo "no changes to commit for $crate"
    continue
  fi

  git commit -m "vendor: add $name"
  git push origin HEAD
done
