#!/bin/bash
set -euxE

clear
npx cucumber-js features/**/*.feature \
	--config cucumber.yaml \
	--require-module tsx \
 	--require features/support/setup.ts \
 	--require features/steps/**/*.ts \
	-f progress-bar \
	--parallel "$(nproc)" \
	--fail-fast

# | sed 's@"$PWD"@//g'
# npx cucumber-js --require-module tsx --require features/steps/**/*.ts
