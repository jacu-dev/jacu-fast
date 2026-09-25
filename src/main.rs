use clap::{Parser, Subcommand, ValueEnum};
use jacu_fast::{
    capabilities, clean, invalid_output, prepare, report, verify, verify_hook, Checkpoint, HookRequest,
    PrepareRequest, ReportFormat, ReportRequest, VerifyRequest,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "jacu",
    version,
    about = "Prepare, verify, and report one coding task."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inventory the repository and bind a task contract.
    Prepare {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        request_file: PathBuf,
        #[arg(long)]
        audit: bool,
        #[arg(long)]
        contract_file: Option<PathBuf>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long, value_enum, default_value_t = FormatArg::Json)]
        format: FormatArg,
    },
    /// Run the next check, the delivery set, or a no-exec hook evaluation.
    Verify {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        session: String,
        #[arg(long, value_enum, default_value_t = CheckpointArg::Delivery)]
        checkpoint: CheckpointArg,
        #[arg(long)]
        hook: Option<String>,
        #[arg(long, value_enum, default_value_t = FormatArg::Json)]
        format: FormatArg,
    },
    /// Print the recorded result for the current candidate.
    Report {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        session: String,
        #[arg(long, value_enum, default_value_t = FormatArg::Json)]
        format: FormatArg,
    },
    /// Remove expired Jacu-owned diagnostics. Repository files stay in place.
    Clean {
        #[arg(long)]
        repo: PathBuf,
    },
    /// Report the platform and the commands this binary actually provides.
    Capabilities,
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Json,
    Markdown,
}

#[derive(Clone, Copy, ValueEnum)]
enum CheckpointArg {
    Iteration,
    Delivery,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            use clap::error::ErrorKind;
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return ExitCode::SUCCESS;
            }
            let output = invalid_output("cli", &error.to_string());
            println!("{}", output.body);
            return ExitCode::from(4);
        }
    };
    let output = match cli.command {
        Commands::Prepare {
            repo,
            request_file,
            audit,
            contract_file,
            session,
            format,
        } => {
            if !matches!(format, FormatArg::Json) {
                invalid_output("prepare", "prepare only writes json")
            } else {
                prepare(PrepareRequest {
                    repo,
                    request_file,
                    audit,
                    contract_file,
                    session,
                })
            }
        }
        Commands::Verify {
            repo,
            session,
            checkpoint,
            hook,
            format,
        } => {
            if !matches!(format, FormatArg::Json) {
                invalid_output("verify", "verify only writes json")
            } else if hook.is_some() {
                verify_hook(HookRequest { repo, session })
            } else {
                verify(VerifyRequest {
                    repo,
                    session,
                    checkpoint: match checkpoint {
                        CheckpointArg::Iteration => Checkpoint::Iteration,
                        CheckpointArg::Delivery => Checkpoint::Delivery,
                    },
                })
            }
        }
        Commands::Report {
            repo,
            session,
            format,
        } => report(ReportRequest {
            repo,
            session,
            format: match format {
                FormatArg::Json => ReportFormat::Json,
                FormatArg::Markdown => ReportFormat::Markdown,
            },
        }),
        Commands::Clean { repo } => clean(repo),
        Commands::Capabilities => capabilities(),
    };
    println!("{}", output.body);
    ExitCode::from(output.exit_code as u8)
}
