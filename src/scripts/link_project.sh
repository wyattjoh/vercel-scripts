#!/usr/bin/env zsh

# @vercel.name Link Project (vercel link)
# @vercel.description Link the project to the vercel/vercel repo
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo
# @vercel.opt { "name": "USE_LOCAL_VERCEL_CLI", "description": "Use the local vercel CLI", "type": "boolean", "default": false }
# @vercel.stdin inherit

set -e

if [ "$USE_LOCAL_VERCEL_CLI" = "true" ]; then
  alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"
fi

vercel link
vercel pull