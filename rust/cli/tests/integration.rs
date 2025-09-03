// Root integration test crate that wires submodules

mod helpers; // looks for rust/cli/tests/helpers/mod.rs
mod integration { // groups files under tests/integration/
    mod cli_basic; // rust/cli/tests/integration/cli_basic.rs
    mod helpers_temp_files; // rust/cli/tests/integration/helpers_temp_files.rs (2.2 Red)
}
