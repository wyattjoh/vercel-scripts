#! /bin/bash

# @vercel.name Build Next.js
# @vercel.description Build the Next.js project
# @vercel.arg VERCEL_NEXT_DIRECTORY The directory for the vercel/next.js repo
# @vercel.opt { "name": "VERCEL_NEXT_WORKTREE", "description": "Select Next.js worktree to build", "type": "worktree", "baseDirArg": "VERCEL_NEXT_DIRECTORY", "default": null, "optional": true }

set -e

# Use VERCEL_NEXT_WORKTREE if set, otherwise use VERCEL_NEXT_DIRECTORY
NEXT_PROJECT_PATH=${VERCEL_NEXT_WORKTREE:-$VERCEL_NEXT_DIRECTORY}

pushd $NEXT_PROJECT_PATH
  pnpm build --filter next
popd