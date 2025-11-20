#!/bin/bash
set -euxE
npx vitest run --passWithNoTests __tests__/validator.test.ts &
npx vitest run --passWithNoTests __tests__/mock_server.test.ts &
# npm run test  --passWithNoTests




