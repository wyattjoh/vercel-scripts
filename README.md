# Vercel Scripts

This is a collection of scripts to help with building and deploying a Vercel
project while developing Next.js.

## Prerequisites

- `pnpm` installed
- `node` installed
- `zsh` installed
- `brew` installed
- `jq` installed

## Setup

1. Add the `./bin` directory to your `PATH` environment variable.
2. Run `vss` from the root of the project to run the script selector.

## Available Scripts

- `./scripts/build_next.sh` - Builds the Next.js project.
- `./scripts/build_vercel.sh` - Builds the Vercel CLI.
- `./scripts/link_next.sh` - Link the local Next.js package as the project's dependency
- `./scripts/build_project.sh` - Builds the Vercel project.
- `./scripts/deploy_project.sh` - Deploys the Vercel project.
