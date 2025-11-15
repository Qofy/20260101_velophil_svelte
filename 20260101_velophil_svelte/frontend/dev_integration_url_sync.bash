#!/bin/bash
set -euxE
clear
# bunx vitest run __tests__/integration.test.ts


# Force Vitest to run in non-watch mode and exit on success/failure
npx vitest run  \
  --config ./vitest.config.ts \
  --passWithNoTests \
  __tests__/integration_url.test.ts \
  --bail 1 \
  --clearScreen \
  --globals
