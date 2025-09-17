// Root integration test crate that wires submodules

mod helpers; // looks for rust/cli/tests/helpers/mod.rs
mod integration {
    // groups files under tests/integration/
    mod assertions_basic; // rust/cli/tests/integration/assertions_basic.rs (2.3)
    mod cli_basic; // rust/cli/tests/integration/cli_basic.rs
    mod config_precedence; // rust/cli/tests/integration/config_precedence.rs (3.2)
    mod evaluation_basic; // rust/cli/tests/integration/evaluation_basic.rs (6.2)
    mod file_corruption_recovery; // rust/cli/tests/integration/file_corruption_recovery.rs (5.2)
    mod file_dir_processing; // rust/cli/tests/integration/file_dir_processing.rs (5.3)
    mod file_io_basic; // rust/cli/tests/integration/file_io_basic.rs (5.1)
    mod game_logic;
    mod helpers_temp_files; // rust/cli/tests/integration/helpers_temp_files.rs (2.2 Red)
    mod simulation_basic; // rust/cli/tests/integration/simulation_basic.rs (6.1)
                          // rust/cli/tests/integration/game_logic.rs (B 4.2)
}
