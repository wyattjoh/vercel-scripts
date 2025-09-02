#!/usr/bin/env zsh

# @vercel.name Link Project (vercel link)
# @vercel.description Link the project to the vercel/vercel repo
# @vercel.arg VERCEL_VERCEL_DIRECTORY The directory for the vercel/vercel repo
# @vercel.stdin inherit

set -e

alias vercel="node ${VERCEL_VERCEL_DIRECTORY}/packages/cli/dist/index.js"

vercel link
vercel pull