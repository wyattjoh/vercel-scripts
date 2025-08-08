#!/usr/bin/env zsh

set -e

# Get the root directory of the project.
export VS_ROOT=$(dirname "$(dirname "$(realpath "$0")")")

# Get the script to run.
SCRIPT_NAME=$1

# If no script is provided, exit.
if [ -z "$SCRIPT_NAME" ]; then
  echo "No script provided"
  exit 1
fi

# If the script is not a file, exit.
if [ ! -f "${VS_ROOT}/src/scripts/${SCRIPT_NAME}" ]; then
  echo "Script ${SCRIPT_NAME} not found"
  exit 1
fi

################################################################################
# Setup the functions.
################################################################################

function backup-file() {
  echo "Backing up $1 to $1.bak"
  cp $1 $1.bak
}

function restore-file() {
  echo "Restoring $1 from $1.bak"
  mv $1.bak $1
}

function update-package-json() {
  echo "Updating $1 to $2"
  jq ".dependencies[\"$1\"] = \"$2\"" package.json > package.json.tmp
  mv package.json.tmp package.json
}

################################################################################
# Run the script.
################################################################################

. "${VS_ROOT}/src/scripts/${SCRIPT_NAME}"