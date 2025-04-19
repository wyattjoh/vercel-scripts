#!/bin/bash

# @vercel.name Build Vercel CLI
# @vercel.description Build the vercel CLI
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo

set -e

pushd $VERCEL_VERCEL_DIRECTORY
  # Replace the version of @vercel/next in the package.json.
  pushd packages/cli
    backup-file package.json
    update-package-json "@vercel/next" "workspace:*"
  popd


  backup-file pnpm-lock.yaml
  pnpm install --prefer-offline
  restore-file pnpm-lock.yaml
  
  pnpm build

  # Restore the package.json file
  pushd packages/cli
    restore-file package.json
  popd
popd