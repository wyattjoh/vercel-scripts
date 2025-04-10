#! /bin/bash

# @vercel.name Build Next.js
# @vercel.description Build the Next.js project
# @vercel.arg VERCEL_NEXT_DIRECTORY The directory for the vercel/next.js repo

set -e

pushd $VERCEL_NEXT_DIRECTORY
  pnpm build --filter next
popd