//! Compatibility entry point for the reproducible strategy matrix.

use std::{env, process::ExitCode};

use schnapsen::{
    core::rules::GameVariant,
    testing::simulation::{self, MatchSetup, RunConfig, PAIRINGS},
};

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let count = args
        .next()
        .map(|arg| arg.parse::<u64>())
        .transpose()
        .map_err(|_| "invalid_seed_count")?
        .unwrap_or(32);
    let first = args
        .next()
        .map(|arg| arg.parse::<u64>())
        .transpose()
        .map_err(|_| "invalid_first_seed")?
        .unwrap_or(1);
    if count == 0 {
        return Err("invalid_seed_count".into());
    }
    if args.next().is_some() {
        return Err("too_many_arguments".into());
    }

    let setup = MatchSetup::default();
    for &profile in GameVariant::default().profiles() {
        for (left, right) in PAIRINGS {
            let report = simulation::simulate(&RunConfig {
                setup: MatchSetup {
                    profile,
                    strategies: [left, right],
                    ..setup
                },
                first_seed: first,
                seed_count: count,
                mirror: true,
                validate_each_step: false,
            })
            .map_err(|error| error.to_string())?;
            println!("{}", report.legacy_json());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", serde_json::json!({"error": error}));
            ExitCode::FAILURE
        }
    }
}
