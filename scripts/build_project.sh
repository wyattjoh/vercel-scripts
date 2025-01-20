#!/bin/bash

# @vercel.name Build Project
# @vercel.description Build the project
# @vercel.after ./package_next.sh ./package_vercel.sh ./link_next.sh
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo

set -e

alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"

vercel build --prod