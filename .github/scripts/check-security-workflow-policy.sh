#!/bin/sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
workflow=${1:-"${script_dir}/../workflows/security.yml"}

concurrency_value() {
  awk -v key="$1" '
    $0 == "concurrency:" { in_concurrency = 1; next }
    in_concurrency && $0 ~ "^  " key ": " {
      sub("^  " key ": ", "")
      print
      exit
    }
    in_concurrency && /^[^ ]/ { exit }
  ' "${workflow}"
}

group=$(concurrency_value group)
cancel_in_progress=$(concurrency_value cancel-in-progress)

expected_group='security-audit-${{ github.event_name }}-${{ github.ref }}'

if [ "${group}" != "${expected_group}" ]; then
  echo "security concurrency must isolate event purpose and ref" >&2
  echo "expected: ${expected_group}" >&2
  echo "actual:   ${group:-<missing>}" >&2
  exit 1
fi

if [ "${cancel_in_progress}" != "true" ]; then
  echo "security concurrency must cancel duplicate runs" >&2
  echo "expected: true" >&2
  echo "actual:   ${cancel_in_progress:-<missing>}" >&2
  exit 1
fi
