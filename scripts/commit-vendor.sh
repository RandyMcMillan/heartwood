#!/bin/sh
set -eu

recursive=0

while [ $# -gt 0 ]; do
  case "$1" in
    -r|--recursive)
      recursive=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
usage: scripts/commit-vendor.sh [--recursive] [vendor/<crate> ...]

Without --recursive, the script commits the provided vendor crates or, if no
crates are given, every untracked top-level vendor crate.

With --recursive, the script walks every vendored crate under vendor/ and
commits them one by one.
EOF
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "error: unknown option: $1" >&2
      exit 1
      ;;
    *)
      break
      ;;
  esac
done

commit_one() {
  crate=$1

  case "$crate" in
    vendor/*) ;;
    *) crate="vendor/$crate" ;;
  esac

  if [ ! -d "$crate" ]; then
    echo "error: missing directory: $crate" >&2
    exit 1
  fi

  label=${crate#vendor/}

  git add "$crate"

  if git diff --cached --quiet -- "$crate"; then
    echo "no changes to commit for $crate"
    return 0
  fi

  git commit -m "vendor: add $label"
  git push origin HEAD
}

if [ "$recursive" -eq 1 ]; then
  set -- $(find vendor -type f -name Cargo.toml | sed 's#/Cargo.toml$##' | awk -F/ '{ print NF "\t" $0 }' | sort -rn -k1,1 | cut -f2-)
elif [ $# -eq 0 ]; then
  set -- $(git ls-files --others --exclude-standard vendor | cut -d/ -f2 | sort -u)
fi

if [ $# -eq 0 ]; then
  echo "no untracked vendor crates found"
  exit 0
fi

for crate in "$@"; do
  commit_one "$crate"
done
