#!/bin/bash
set -euxE
clear
# bunx vitest run --passWithNoTests __tests__/integration.test.ts --watch
# bun run vitest  --passWithNoTests  __tests__/integration.test.ts --watch

# Recompile Sass and TypeScript on change and restart server
npx nodemon \
  --watch style.scss \
  --watch src/ \
  --watch index.html \
  --ext jsx,js,tsx,ts,css,scss,html \
  --ignore dist/ \
  --watch  __tests__/ \
  --exec './dev_integration_url.bash'  2>&1 | sed 's@'"$(pwd)"'@@g'
