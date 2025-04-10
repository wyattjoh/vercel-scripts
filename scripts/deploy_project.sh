#!/bin/bash

# @vercel.name Deploy Project
# @vercel.description Deploy the project
# @vercel.after ./build_project.sh
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo

set -e

alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"

vercel --prebuilt --force --prod