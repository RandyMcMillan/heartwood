#!/bin/bash

set -xeuo pipefail

if [ "${RADICLE_CI_BROKER_WEBROOT:-unset}" != unset ]; then
	echo web root set, publishing arch doc there
	install -d -m 0755 "$RADICLE_CI_BROKER_WEBROOT"
	install -m 0644 ./*.html "$RADICLE_CI_BROKER_WEBROOT"
else
	echo web root is not set, not publishing arch doc
fi
