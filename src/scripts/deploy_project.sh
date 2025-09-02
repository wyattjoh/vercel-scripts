#!/usr/bin/env zsh

# @vercel.name Deploy Project (vc deploy --prebuilt)
# @vercel.description Deploy the project using the prebuilt build.
# @vercel.after ./build_project.sh
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo
# @vercel.opt { "name": "VERCEL_BUILD_PRODUCTION", "description": "Build the project in production mode", "type": "boolean", "default": false }

set -e

alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"

if [ "$VERCEL_BUILD_PRODUCTION" = "true" ]; then
  echo "Deploying project in production mode..."
  export VERCEL_DEPLOYMENT_ORIGIN=$(vercel --prebuilt --force --prod)
else
  echo "Deploying project in preview mode..."
  export VERCEL_DEPLOYMENT_ORIGIN=$(vercel --prebuilt --force)
fi
