#!/bin/bash

# @vercel.name Link Next.js
# @vercel.description Install the local Next.js package as the project's dependency
# @vercel.after ./build_next.sh
# @vercel.arg VERCEL_NEXT_DIRECTORY The directory for the vercel/next.js repo

set -e

pushd $VERCEL_NEXT_DIRECTORY/packages/next
  pnpm link --global
popd

pnpm link --global next