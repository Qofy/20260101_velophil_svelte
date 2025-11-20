#!/bin/bash
set -euxE

npx cucumber-js features/**/*.feature \
	--config cucumber.yaml \
	--require-module tsx \
 	--require features/support/setup.ts \
 	--require features/steps/**/*.ts

# | sed 's@"$PWD"@//g'
# npx cucumber-js --require-module tsx --require features/steps/**/*.ts
