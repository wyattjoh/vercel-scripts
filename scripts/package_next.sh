#!/bin/bash

# @vercel.name Package Next.js
# @vercel.description Package the Next.js project
# @vercel.after ./build_next.sh
# @vercel.arg NEXT_DEV_UTILS_DIRECTORY The directory for the wyattjoh/next-dev-utils repo
# @vercel.arg VERCEL_NEXT_DIRECTORY The directory for the vercel/next.js repo
# @vercel.opt { "name": "VERCEL_NEXT_WORKTREE", "description": "Select Next.js worktree to link", "type": "worktree", "baseDirArg": "VERCEL_NEXT_DIRECTORY", "default": null, "optional": true }

set -e

# Use VERCEL_NEXT_WORKTREE if set, otherwise use VERCEL_NEXT_DIRECTORY
export NEXT_PROJECT_PATH=${VERCEL_NEXT_WORKTREE:-$VERCEL_NEXT_DIRECTORY}

# Pack up Next.js and install the specified version.
nu pack-next --install