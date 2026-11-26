use std::{
    collections::BTreeMap,
    env,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
};

use schnapsen::{
    core::{
        game::Player,
        history::HistoryCoverage,
        rules::{Difficulty, GameVariant, RuleProfile, SchneiderRule},
    },
    save::SaveStore,
    testing::{
        case::{replay_history, CaseStart, TestCase},
        simulation::{self, MatchSetup, RunConfig},
        statistics::Report,
        validation,
    },
};
use serde::{de::DeserializeOwned, Serialize};

const HELP: &str = "schnapsen_lab simulate [OPTIONS]
schnapsen_lab matrix [OPTIONS]
schnapsen_lab record --seed N --output CASE.json [OPTIONS]
schnapsen_lab replay CASE.json
schnapsen_lab validate-case CASE.json
schnapsen_lab validate-save SAVE.json
schnapsen_lab from-save SAVE.json --output CASE.json [--seed N]

Simulation: --seed N (first seed; default 1), --count N (seeds; default 1)
             --variant austrian_schnapsen
             --profile soft|sharp, --schneider double|single
             --local easy|medium|hard, --opponent easy|medium|hard
             --forehand local|opponent, --mirror, --validate
             --output REPORT.json, --failure PREFIX
Matrix:      --seed N, --count N, --variant ID, --forehand PLAYER,
             --output REPORT.json, --failure PREFIX
             All profiles, both scoring settings and all pairings.
             Seats are mirrored and every action is validated.
Record:      Same setup options as simulate, plus --validate and --failure.
             --mirror records the swapped seating.
Validation and replay never modify the input file.
";

#[derive(Default)]
struct Options {
    values: BTreeMap<String, String>,
    mirror: bool,
    validate: bool,
}

impl Options {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.peekable();
        while let Some(name) = args.next() {
            let flag = match name.as_str() {
                "--mirror" => &mut options.mirror,
                "--validate" => &mut options.validate,
                _ if name.starts_with("--") => {
                    let value = args
                        .next_if(|value| !value.starts_with("--"))
                        .ok_or_else(|| format!("missing value for {name}"))?;
                    if options.values.insert(name.clone(), value).is_some() {
                        return Err(format!("duplicate option {name}"));
                    }
                    continue;
                }
                _ => return Err(format!("unexpected argument {name}")),
            };
            if std::mem::replace(flag, true) {
                return Err(format!("duplicate option {name}"));
            }
        }
        Ok(options)
    }

    fn number(&mut self, key: &str, default: u64) -> Result<u64, String> {
        self.values.remove(key).map_or(Ok(default), |value| {
            value.parse().map_err(|_| format!("invalid {key}: {value}"))
        })
    }

    fn id<T: DeserializeOwned>(&mut self, key: &str, default: T) -> Result<T, String> {
        self.values.remove(key).map_or(Ok(default), |value| {
            serde_json::from_value(serde_json::Value::String(value.clone()))
                .map_err(|_| format!("invalid {key}: {value}"))
        })
    }

    fn path(&mut self, key: &str) -> Option<PathBuf> {
        self.values.remove(key).map(PathBuf::from)
    }

    fn finish(self) -> Result<(), String> {
        if let Some(key) = self.values.keys().next() {
            return Err(format!("unknown option {key}"));
        }
        if self.mirror || self.validate {
            return Err("unexpected --mirror or --validate".into());
        }
        Ok(())
    }
}

fn new_file(path: &Path) -> Result<BufWriter<File>, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map(BufWriter::new)
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn output_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut writer = new_file(path)?;
    serde_json::to_writer_pretty(&mut writer, value).map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn write_case(path: &Path, case: &TestCase) -> Result<(), String> {
    let mut writer = new_file(path)?;
    case.write(&mut writer).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn game_setup(options: &mut Options) -> Result<MatchSetup, String> {
    let variant = options.id("--variant", GameVariant::AustrianSchnapsen)?;
    let profile = options.id("--profile", RuleProfile::Soft)?;
    if !variant.profiles().contains(&profile) {
        return Err("selected profile does not support the variant".into());
    }
    Ok(MatchSetup {
        variant,
        profile,
        schneider: options.id("--schneider", SchneiderRule::Double)?,
        strategies: [
            options.id("--local", Difficulty::Easy)?,
            options.id("--opponent", Difficulty::Hard)?,
        ],
        forehand: options.id("--forehand", Player::Local)?,
    })
}

fn report_failure(failure: &simulation::Failure, prefix: &Path) -> String {
    let report = prefix.with_extension("failure.json");
    let case = prefix.with_extension("case.json");
    let message = format!(
        "simulation failed (seed {}, hand {}, step {}, mirrored {}): {:?}",
        failure.seed, failure.hand, failure.step, failure.mirrored, failure.reason,
    );
    let setup = failure.setup;
    let variant = serde_json::json!(setup.variant);
    let profile = serde_json::json!(setup.profile);
    let schneider = serde_json::json!(setup.schneider);
    let forehand = serde_json::json!(setup.forehand);
    let mirror = if failure.mirrored { " --mirror" } else { "" };
    let retry = format!(
        "schnapsen_lab record --seed {} --variant {variant} --profile {profile} --schneider {schneider} --local {} --opponent {} --forehand {forehand}{mirror} --validate --output retry.case.json --failure retry",
        failure.seed, setup.strategies[0].id(), setup.strategies[1].id(),
    );
    let exported = output_json(&report, failure).and_then(|()| {
        if let Some(recorded) = &failure.case {
            write_case(&case, recorded)?;
        }
        Ok(())
    });
    match exported {
        Ok(()) => {
            let mut message = format!("{message}\nstate and history: {}", report.display());
            if failure.case.is_some() {
                message.push_str(&format!(
                    "\nreplay: schnapsen_lab replay {}",
                    case.display()
                ));
            }
            format!("{message}\nsimulate again: {retry}")
        }
        Err(error) => format!("{message}\nexport failed: {error}\nsimulate again: {retry}"),
    }
}

fn record(options: &mut Options, setup: MatchSetup, seed: u64) -> Result<(), String> {
    let output = options.path("--output").ok_or("record requires --output")?;
    let failure = options
        .path("--failure")
        .unwrap_or_else(|| "simulation".into());
    let mirror = std::mem::take(&mut options.mirror);
    let validate = std::mem::take(&mut options.validate);
    std::mem::take(options).finish()?;
    let completed = simulation::play_one(setup, seed, mirror, validate)
        .map_err(|error| report_failure(&error, &failure))?;
    write_case(&output, &completed.case)?;
    println!("recorded seed {seed}: {}", output.display());
    Ok(())
}

fn simulate(command: &str, mut options: Options) -> Result<(), String> {
    let seed = options.number("--seed", 1)?;
    if command == "record" {
        let setup = game_setup(&mut options)?;
        return record(&mut options, setup, seed);
    }
    let count = options.number("--count", 1)?;
    if count == 0 || seed.checked_add(count - 1).is_none() {
        return Err("invalid seed range".into());
    }
    let output = options.path("--output");
    let failure = options
        .path("--failure")
        .unwrap_or_else(|| "simulation".into());
    let (setups, mirror, validate) = if command == "matrix" {
        let variant = options.id("--variant", GameVariant::AustrianSchnapsen)?;
        let forehand = options.id("--forehand", Player::Local)?;
        let mut setups = Vec::new();
        for &profile in variant.profiles() {
            for schneider in [SchneiderRule::Double, SchneiderRule::Single] {
                for (local, opponent) in simulation::PAIRINGS {
                    setups.push(MatchSetup {
                        variant,
                        profile,
                        schneider,
                        forehand,
                        strategies: [local, opponent],
                    });
                }
            }
        }
        (setups, true, true)
    } else {
        (
            vec![game_setup(&mut options)?],
            std::mem::take(&mut options.mirror),
            std::mem::take(&mut options.validate),
        )
    };
    options.finish()?;
    let started = std::time::Instant::now();
    let mut reports: Vec<Report> = Vec::with_capacity(setups.len());
    for setup in setups {
        let report = simulation::simulate(&RunConfig {
            setup,
            first_seed: seed,
            seed_count: count,
            mirror,
            validate_each_step: validate,
        })
        .map_err(|error| report_failure(&error, &failure))?;
        println!("{report}");
        reports.push(report);
    }
    let elapsed = started.elapsed().as_secs_f64();
    println!("total elapsed {elapsed:.3} s");
    if let Some(output) = output {
        output_json(
            &output,
            &serde_json::json!({
                "format_version": 1,
                "wall_time_seconds": elapsed,
                "reports": reports,
            }),
        )?;
    }
    Ok(())
}

fn read_save(path: PathBuf) -> Result<schnapsen::core::match_play::Match, String> {
    let snapshot = SaveStore::new(path.clone())
        .load()
        .map_err(|error| format!("{}: {error}", path.display()))?
        .ok_or_else(|| format!("{}: save file not found", path.display()))?;
    validation::validate(&snapshot.session).map_err(|error| error.to_string())?;
    let state = snapshot.session;
    if state.history().coverage() == HistoryCoverage::Complete {
        let replayed = replay_history(state.history()).map_err(|error| error.to_string())?;
        if replayed != state {
            return Err("recorded history does not reproduce the saved state".into());
        }
    }
    Ok(state)
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            Ok(())
        }
        "simulate" | "matrix" | "record" => simulate(&command, Options::parse(args)?),
        "replay" | "validate-case" | "validate-save" | "from-save" => {
            let input = PathBuf::from(args.next().ok_or("missing input file")?);
            let mut options = Options::parse(args)?;
            if matches!(command.as_str(), "replay" | "validate-case") {
                options.finish()?;
                let reader = BufReader::new(
                    File::open(&input).map_err(|error| format!("{}: {error}", input.display()))?,
                );
                let case = TestCase::read(reader).map_err(|error| error.to_string())?;
                let state = case.replay().map_err(|error| error.to_string())?;
                println!(
                    "verified {} actions, {} completed hands, result {:?}",
                    case.actions.len(),
                    state.completed_hands(),
                    state.result()
                );
            } else if command == "validate-save" {
                options.finish()?;
                let state = read_save(input)?;
                println!(
                    "valid save, {} completed hands, result {:?}",
                    state.completed_hands(),
                    state.result()
                );
            } else {
                let seed = options.number("--seed", 1)?;
                let output = options
                    .path("--output")
                    .ok_or("from-save requires --output")?;
                options.finish()?;
                let state = read_save(input)?;
                let case = TestCase::new(CaseStart::Snapshot {
                    state: Box::new(state),
                    continuation_seed: seed,
                });
                write_case(&output, &case)?;
                println!("exported initial test state: {}", output.display());
            }
            Ok(())
        }
        _ => Err(format!("unknown command {command}; see --help")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
