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

1. `pnpm install && pnpm build`
2. Add the `./bin` directory to your `PATH` environment variable.
3. Run `vss` from the root of the project to run the script selector.

New scripts can be added by adding a new script file to the `scripts` directory
and making it executable with `chmod +x <script-name>.sh`. Annotations with the
`@vercel` prefix will be used to configure the script selector, refer to
existing scripts for examples.

Any arguments required by the script will be passed to the script via
environment variables and persisted to the `args.json` file in the root of the
vercel scripts directory.

## Available Scripts

- `./scripts/build_next.sh` - Builds the Next.js project.
- `./scripts/build_project.sh` - Builds the a project using the Vercel CLI.
- `./scripts/build_vercel.sh` - Builds the Vercel CLI.
- `./scripts/deploy_project.sh` - Deploys the Vercel project.
- `./scripts/link_local_next.sh` - Link the local Next.js package as the project's dependency
- `./scripts/package_next.sh` - Packages the Next.js project.
