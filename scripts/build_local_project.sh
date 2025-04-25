#!/bin/bash

# @vercel.name Build Project for Local (next build)
# @vercel.description Build the project for local testing (not deployment).
# @vercel.after ./link_local_next.sh

set -e

# Build the project
pnpm next build
