#!/usr/bin/env zsh

set -e

# Get the script to run.
SCRIPT_PATHNAME=$1
SCRIPT_NAME=$(basename "${SCRIPT_PATHNAME}")

# If no script is provided, exit.
if [ -z "$SCRIPT_NAME" ]; then
  echo "No script provided"
  exit 1
fi

# If the script is not a file, exit.
if [ ! -f "${SCRIPT_PATHNAME}" ]; then
  echo "Script ${SCRIPT_PATHNAME} not found"
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

# Capture exported variables before script execution
export -p | grep -E '(declare -x |export )[A-Za-z_][A-Za-z0-9_]*=' | sort > "$VSS_PRE_ENV_FILE"

# Debug: show pre-execution count if VSS_DEBUG is set
if [ -n "$VSS_DEBUG" ]; then
    echo "DEBUG: Pre-execution exports: $(wc -l < "$VSS_PRE_ENV_FILE")" >&2
fi

# Source and run the script
. "${SCRIPT_PATHNAME}"

# Capture exported variables after script execution  
export -p | grep -E '(declare -x |export )[A-Za-z_][A-Za-z0-9_]*=' | sort > "$VSS_POST_ENV_FILE"

# Debug: show post-execution count and diff if VSS_DEBUG is set
if [ -n "$VSS_DEBUG" ]; then
    echo "DEBUG: Post-execution exports: $(wc -l < "$VSS_POST_ENV_FILE")" >&2
    echo "DEBUG: New/changed exports:" >&2
    comm -13 "$VSS_PRE_ENV_FILE" "$VSS_POST_ENV_FILE" >&2
fi

