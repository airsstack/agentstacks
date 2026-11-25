#!/bin/bash
ev="$1"; out="$2/$ev.$$"
{ echo "PWD=$(pwd)"; echo "--- ENV"; env | sort; echo "--- STDIN"; cat; } > "$out"
