#!/bin/bash

# @vercel.name Build Project for Deploy (vc build)
# @vercel.description Build the project for deployment.
# @vercel.after ./package_next.sh ./package_vercel.sh ./link_next.sh
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo
# @vercel.opt { "name": "VERCEL_BUILD_PRODUCTION", "description": "Build the project in production mode", "type": "boolean", "default": false }

set -e

alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"

if [ "$VERCEL_BUILD_PRODUCTION" = "true" ]; then
  vercel build --prod
else
  vercel build
fi
