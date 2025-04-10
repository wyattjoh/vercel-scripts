#!/bin/bash

# @vercel.name Deploy Project
# @vercel.description Deploy the project
# @vercel.after ./build_project.sh
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo
# @vercel.opt { "name": "VERCEL_BUILD_PRODUCTION", "description": "Build the project in production mode", "type": "boolean", "default": false }

set -e

alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"

if [ "$VERCEL_BUILD_PRODUCTION" = "true" ]; then
  vercel --prebuilt --force --prod
else
  vercel --prebuilt --force
fi
