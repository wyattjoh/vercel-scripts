#!/bin/bash

# @vercel.name Start Local Project (next start)
# @vercel.description Start the project for local testing (not deployment).
# @vercel.after ./build_local_project.sh

set -e

# Start the project
pnpm next start
