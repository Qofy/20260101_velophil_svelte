#!/bin/bash
set -euxE
typeset -i _out=0

function run_tests() {
  local -i _comp=0
  local -i _test=0
  local -i _cucu=0
  local -i _int1=0
  local -i _int2=0
  local -i _ret=0

  # Compile everything first
  ./dev_compile.bash
  _comp=$?
  # wait

  # Run tests
  ./dev_test.bash &
  _test=$?
  # wait

  ./dev_cucumber.bash &
  _cucu=$?
  # wait

  ./dev_integration.bash &
  _int1=$?
  # wait

  ./dev_integration_url.bash &
  _int2=$?
  # wait

  _ret=$(( _comp + _test + _cucu + _int1 + _int2 ))
  wait

  echo $_ret
  wait
  # Start the dev server on a defined port
  # PORT=${1:-3000}
  # npx serve --port $PORT --silent
  return $_ret
}

run_tests

