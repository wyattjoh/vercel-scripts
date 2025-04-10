#!/bin/bash

# @vercel.name Package Next.js
# @vercel.description Package the Next.js project
# @vercel.after ./build_next.sh
# @vercel.arg VERCEL_NEXT_DIRECTORY The directory for the vercel/next.js repo
# @vercel.arg NEXT_DEV_UTILS_DIRECTORY The directory for the wyattjoh/next-dev-utils repo

set -e

alias nu="fnm exec --using=v20 node $NEXT_DEV_UTILS_DIRECTORY/packages/cli/dist/cli.js"

pushd $VERCEL_NEXT_DIRECTORY
  NEXT_VERSION=$(nu pack-next --json)
  echo "NEXT_VERSION: $NEXT_VERSION"
popd

pnpm install next@$NEXT_VERSION