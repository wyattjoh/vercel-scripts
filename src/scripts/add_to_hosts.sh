#!/usr/bin/env zsh

# @vercel.name Add to /etc/hosts
# @vercel.description Add the proxy IP address and the deployment origin to the hosts file.
# @vercel.requires ./deploy_project.sh VERCEL_DEPLOYMENT_ORIGIN
# @vercel.opt { "name": "PROXY_IP_ADDRESS", "description": "The IP address of the proxy to add to the hosts file", "type": "string", "default": "127.0.0.1", "pattern": "\\d+\\.\\d+\\.\\d+\\.\\d+", "pattern_help": "The proxy must be a valid IP address" }
# @vercel.stdin inherit

set -e

VERCEL_DEPLOYMENT_HOST=${VERCEL_DEPLOYMENT_ORIGIN#https://}

echo "🔧 Adding $PROXY_IP_ADDRESS -> $VERCEL_DEPLOYMENT_HOST to /etc/hosts"
echo "$PROXY_IP_ADDRESS\t$VERCEL_DEPLOYMENT_HOST" | sudo tee -a /etc/hosts