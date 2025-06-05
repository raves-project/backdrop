#!/bin/bash
set -e

# check for unused dependencies
cargo machete

# ensure readme is up-to-date
cargo rdme --check

# check dependency security
cargo deny check

# run our test suite
cargo nextest run

# run doc tests
cargo test --doc
