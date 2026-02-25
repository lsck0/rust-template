#!/usr/bin/env bash

set -euo pipefail

instance_name="ci-$1"

if [ -f "$GITHUB_WORKSPACE/cargo-generate.toml" ]; then
	cargo generate --path "$GITHUB_WORKSPACE" --name "$instance_name" --destination "$RUNNER_TEMP" --vcs none --silent
	echo "PROJECT_DIR=$RUNNER_TEMP/$instance_name" >>"$GITHUB_ENV"
else
	echo "PROJECT_DIR=$GITHUB_WORKSPACE" >>"$GITHUB_ENV"
fi
