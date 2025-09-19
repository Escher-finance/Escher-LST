#!/usr/bin/env bash

input="$1"

if [[ "$input" == 0x* ]]; then
  echo -n "${input:2}"
else
  echo -n "$input" | xxd -ps -c 1024 | tr -d '\n'
fi
