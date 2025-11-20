#!/bin/bash
set -euxE

# Recompile Sass and TypeScript on change and restart server
npx nodemon \
  --watch style.scss \
  --watch src/ \
  --watch index.html \
  --ext feature,jsx,js,tsx,ts,css,scss,html \
  --ignore dist/ \
  --watch __tests__/ \
  --watch features/ \
  --watch features/steps \
  --watch features/support \
  --exec './dev_cucumber.bash' 2>&1 | sed 's@'"$(pwd)"'@@g'
