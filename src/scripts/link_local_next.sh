#!/usr/bin/env zsh

# @vercel.name Link Local Next.js
# @vercel.description Install the local Next.js package as the project's dependency
# @vercel.after ./build_next.sh
# @vercel.arg VERCEL_NEXT_DIRECTORY The directory for the vercel/next.js repo
# @vercel.opt { "name": "VERCEL_NEXT_WORKTREE", "description": "Select Next.js worktree to link", "type": "worktree", "baseDirArg": "VERCEL_NEXT_DIRECTORY", "default": null, "optional": true }

set -e

# Use VERCEL_NEXT_WORKTREE if set, otherwise use VERCEL_NEXT_DIRECTORY
NEXT_PROJECT_PATH=${VERCEL_NEXT_WORKTREE:-$VERCEL_NEXT_DIRECTORY}

pnpm install next@file:${NEXT_PROJECT_PATH}/packages/next