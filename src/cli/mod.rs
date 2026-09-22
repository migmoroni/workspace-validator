//! CLI parsing, command dispatch, cancellation, and exit-code translation.

mod inspection;

use crate::{
    config::{self, ValidatedConfig},
    contracts::{config::Config, report::ValidationReport},
    error::ValidatorError,
    execution,
    planning::{self, ValidationPlan},
    reporting::{
        self,
        theme::{PaletteProfile, PresentationProfile},
    },
};
use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use schemars::schema_for;
use std::{
    path::PathBuf,
    process::ExitCode,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Parser)]
#[command(
    name = "workspace-validator",
    version,
    about = "Runs declarative workspace validation pipelines"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Validate {
        /// Group or suite ID; defaults to the configured default group.
        target: Option<String>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
        /// Selects a semantic color palette; bare --color selects standard.
        #[arg(
            long,
            value_enum,
            num_args = 0..=1,
            default_missing_value = "standard",
            require_equals = true,
            value_name = "PALETTE"
        )]
        color: Option<ColorPaletteArgument>,
        /// Selects layout and emphasis independently from the color palette.
        #[arg(long, value_enum, require_equals = true, value_name = "MODE")]
        presentation: Option<PresentationArgument>,
    },
    Check {
        check_id: String,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
        /// Selects a semantic color palette; bare --color selects standard.
        #[arg(
            long,
            value_enum,
            num_args = 0..=1,
            default_missing_value = "standard",
            require_equals = true,
            value_name = "PALETTE"
        )]
        color: Option<ColorPaletteArgument>,
        /// Selects layout and emphasis independently from the color palette.
        #[arg(long, value_enum, require_equals = true, value_name = "MODE")]
        presentation: Option<PresentationArgument>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    List {
        #[arg(long)]
        tree: bool,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Explain {
        #[command(subcommand)]
        target: ExplainTarget,
    },
    Schema {
        #[arg(value_enum)]
        contract: Contract,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Validate {
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ExplainTarget {
    Group {
        group_id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Suite {
        suite_id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Check {
        check_id: String,
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorPaletteArgument {
    Standard,
    HighContrast,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Achromatopsia,
}

#[derive(Clone, Copy, ValueEnum)]
enum PresentationArgument {
    Standard,
    LowVision,
}

impl From<ColorPaletteArgument> for PaletteProfile {
    fn from(value: ColorPaletteArgument) -> Self {
        match value {
            ColorPaletteArgument::Standard => Self::Standard,
            ColorPaletteArgument::HighContrast => Self::HighContrast,
            ColorPaletteArgument::Protanopia => Self::Protanopia,
            ColorPaletteArgument::Deuteranopia => Self::Deuteranopia,
            ColorPaletteArgument::Tritanopia => Self::Tritanopia,
            ColorPaletteArgument::Achromatopsia => Self::Achromatopsia,
        }
    }
}

impl From<PresentationArgument> for PresentationProfile {
    fn from(value: PresentationArgument) -> Self {
        match value {
            PresentationArgument::Standard => Self::Standard,
            PresentationArgument::LowVision => Self::LowVision,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Contract {
    Config,
    Report,
}

/// Parses process arguments, executes the requested command, and returns its exit status.
///
/// This entry point reads the current process arguments and working directory,
/// writes command output to standard output or standard error, and installs a
/// cooperative `Ctrl+C` handler for validation commands. Returned codes follow
/// the stable CLI contract documented in the crate README: `0` for success,
/// `1` for a failed result, `2` for a blocked or skipped result, `3` for usage
/// or configuration errors, `4` for internal failures, and `130` for an
/// interrupted validation.
pub fn run_cli() -> ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if color_value_without_equals(&arguments) {
        eprintln!("error: named --color palettes require --color=<PALETTE>");
        return ExitCode::from(3);
    }
    if presentation_value_without_equals(&arguments) {
        eprintln!("error: named presentation modes require --presentation=<MODE>");
        return ExitCode::from(3);
    }
    let cli = match Cli::try_parse_from(arguments) {
        Ok(value) => value,
        Err(error) => {
            let code = match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 3,
            };
            let _ = error.print();
            return ExitCode::from(code);
        }
    };
    match execute(cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err((error, code)) => {
            eprintln!("workspace-validator: {error}");
            ExitCode::from(code)
        }
    }
}

fn execute(cli: Cli) -> Result<i32, (ValidatorError, u8)> {
    if let Command::Schema { contract } = cli.command {
        let schema = match contract {
            Contract::Config => schema_for!(Config),
            Contract::Report => schema_for!(ValidationReport),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&schema)
                .map_err(|error| (ValidatorError::Internal(error.to_string()), 4))?
        );
        return Ok(0);
    }
    if matches!(
        &cli.command,
        Command::Validate {
            format: Format::Json,
            color: Some(_),
            ..
        } | Command::Validate {
            format: Format::Json,
            presentation: Some(_),
            ..
        } | Command::Check {
            format: Format::Json,
            color: Some(_),
            ..
        } | Command::Check {
            format: Format::Json,
            presentation: Some(_),
            ..
        }
    ) {
        return Err((
            ValidatorError::Usage(
                "--color and --presentation cannot be combined with --format=json".into(),
            ),
            3,
        ));
    }
    let current = std::env::current_dir()
        .map_err(|error| (ValidatorError::Internal(error.to_string()), 4))?;
    let explicit = match &cli.command {
        Command::Validate { config, .. }
        | Command::Check { config, .. }
        | Command::List { config, .. } => config.as_deref(),
        Command::Config {
            command: ConfigCommand::Validate { config },
        } => config.as_deref(),
        Command::Explain { target } => match target {
            ExplainTarget::Group { config, .. }
            | ExplainTarget::Suite { config, .. }
            | ExplainTarget::Check { config, .. } => config.as_deref(),
        },
        Command::Schema { .. } => unreachable!(),
    };
    let validated = config::load(explicit, &current).map_err(|error| (error, 3))?;
    let mut stdout = std::io::stdout().lock();
    match cli.command {
        Command::Config { .. } => {
            inspection::config_validate(&validated, &mut stdout)
                .map_err(|error| (ValidatorError::Internal(error), 4))?;
            Ok(0)
        }
        Command::List { tree, .. } => {
            inspection::list(&validated, tree, &mut stdout)
                .map_err(|error| (ValidatorError::Internal(error), 4))?;
            Ok(0)
        }
        Command::Explain { target } => {
            match target {
                ExplainTarget::Group { group_id, .. } => {
                    inspection::explain_group(&validated, &group_id, &mut stdout).map_err(
                        |details| {
                            (
                                ValidatorError::invalid(
                                    validated.configuration_path(),
                                    details.to_string(),
                                ),
                                3,
                            )
                        },
                    )?
                }
                ExplainTarget::Suite { suite_id, .. } => {
                    inspection::explain_suite(&validated, &suite_id, &mut stdout).map_err(
                        |details| {
                            (
                                ValidatorError::invalid(
                                    validated.configuration_path(),
                                    details.to_string(),
                                ),
                                3,
                            )
                        },
                    )?
                }
                ExplainTarget::Check { check_id, .. } => {
                    inspection::explain_check(&validated, &check_id, &mut stdout).map_err(
                        |details| {
                            (
                                ValidatorError::invalid(
                                    validated.configuration_path(),
                                    details.to_string(),
                                ),
                                3,
                            )
                        },
                    )?
                }
            };
            Ok(0)
        }
        Command::Validate {
            target,
            format,
            color,
            presentation,
            ..
        } => {
            let plan = planning::target(&validated, target.as_deref()).map_err(|details| {
                (
                    ValidatorError::invalid(validated.configuration_path(), details.to_string()),
                    3,
                )
            })?;
            execute_run(&validated, &plan, format, color, presentation)
        }
        Command::Check {
            check_id,
            format,
            color,
            presentation,
            ..
        } => {
            let plan = planning::check(&validated, &check_id).map_err(|details| {
                (
                    ValidatorError::invalid(validated.configuration_path(), details.to_string()),
                    3,
                )
            })?;
            execute_run(&validated, &plan, format, color, presentation)
        }
        Command::Schema { .. } => unreachable!(),
    }
}

fn execute_run(
    validated: &ValidatedConfig,
    plan: &ValidationPlan,
    format: Format,
    color: Option<ColorPaletteArgument>,
    presentation: Option<PresentationArgument>,
) -> Result<i32, (ValidatorError, u8)> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = cancelled.clone();
    // The signal handler only flips an atomic flag; process-group termination
    // remains owned by the executor where child lifecycle is available.
    ctrlc::set_handler(move || signal.store(true, std::sync::atomic::Ordering::SeqCst)).map_err(
        |error| {
            (
                ValidatorError::Internal(format!("cannot install signal handler: {error}")),
                4,
            )
        },
    )?;
    let (outcome, theme, separate_result) = match format {
        Format::Human => {
            let theme = reporting::theme::Theme::resolve(
                color
                    .map(PaletteProfile::from)
                    .unwrap_or(PaletteProfile::Plain),
                presentation
                    .map(PresentationProfile::from)
                    .unwrap_or(PresentationProfile::Standard),
            );
            let mut reporter = reporting::execution::TerminalExecutionReporter::new(theme.clone());
            let separate_result = reporter.is_visible();
            let outcome = execution::run_with_progress(validated, plan, cancelled, &mut reporter);
            // Drop indicatif before writing the lifecycle separator so no
            // pending redraw can consume or move the blank terminal row.
            drop(reporter);
            (outcome, Some(theme), separate_result)
        }
        Format::Json => (execution::run(validated, plan, cancelled), None, false),
    };
    let rendered = match format {
        Format::Human => {
            reporting::result::human::render(&outcome.report, theme.as_ref().expect("human theme"))
        }
        Format::Json => reporting::result::json::render(&outcome.report)
            .map_err(|error| (ValidatorError::Internal(error.to_string()), 4))?,
    };
    if separate_result {
        // Indicatif leaves the cursor at the end of its completed footer. The
        // first newline closes that row; the second creates the visible gap.
        eprintln!("\n");
    }
    println!("{rendered}");
    Ok(if outcome.interrupted {
        130
    } else {
        outcome.report.summary.exit_code()
    })
}

fn color_value_without_equals(arguments: &[std::ffi::OsString]) -> bool {
    arguments.windows(2).any(|pair| {
        pair[0] == "--color"
            && pair[1]
                .to_str()
                .is_some_and(|value| ColorPaletteArgument::from_str(value, true).is_ok())
    })
}

fn presentation_value_without_equals(arguments: &[std::ffi::OsString]) -> bool {
    arguments.windows(2).any(|pair| {
        pair[0] == "--presentation"
            && pair[1]
                .to_str()
                .is_some_and(|value| PresentationArgument::from_str(value, true).is_ok())
    })
}
