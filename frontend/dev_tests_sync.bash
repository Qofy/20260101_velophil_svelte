#!/bin/bash
set -euxE

# Compile everything first
./dev_compile.bash

# Run tests
./dev_test_sync.bash
./dev_cucumber_sync.bash
./dev_integration_sync.bash
./dev_integration_url_sync.bash


# Start the dev server on a defined port
# PORT=${1:-3000}
# yarn run serve --port $PORT --silent
