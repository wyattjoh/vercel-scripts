#!/bin/bash

# @vercel.name Package Next.js
# @vercel.description Package the Next.js project
# @vercel.after ./build_next.sh
# @vercel.arg NEXT_DEV_UTILS_DIRECTORY The directory for the wyattjoh/next-dev-utils repo

set -e

alias nu="fnm exec --using=v20 node $NEXT_DEV_UTILS_DIRECTORY/packages/cli/dist/cli.js"

# Pack up Next.js and install the specified version.
nu pack-next --install