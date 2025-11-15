#!/bin/bash
set -uxeE

echo "▶️ Compiling Sass..."
npx sass style.scss dist/style.css
