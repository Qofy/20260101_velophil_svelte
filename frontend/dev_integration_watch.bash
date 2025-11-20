#!/bin/bash
set -euxE
clear

# npx vitest run __tests__/integration.test.ts

# Recompile Sass and TypeScript on change and restart server
npx nodemon \
  --watch style.scss \
  --watch src/ \
  --watch index.html \
  --ext jsx,js,tsx,ts,css,scss,html \
  --ignore dist/ \
  --watch  __tests__/ \
  --exec './dev_integration.bash'  2>&1 | sed 's@'"$(pwd)"'@@g'
