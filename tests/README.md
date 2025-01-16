# Tests

These are the integration tests for the library. Please add some of your weirdest media to the `assets` directory and write some fun tests to check our parsers.

## Running Tests

Typically, `cargo nextest run` will work just fine.

If you'd like to use the Tokio debugger, however, you'll need to:

1. Install it: `cargo binstall --locked tokio-console`
2. Run the tests with a special compiler flag: `RUSTFLAGS="--cfg tokio_unstable" cargo nextest run`
