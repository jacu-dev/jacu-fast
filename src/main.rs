use clap::{Parser, Subcommand, ValueEnum};
use jacu_fast::{
    invalid_output, prepare, report, verify, Checkpoint, PrepareRequest, ReportFormat, ReportRequest,
    VerifyRequest,
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
    /// Run the next check, or the delivery set.
    Verify {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        session: String,
        #[arg(long, value_enum)]
        checkpoint: CheckpointArg,
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
            format,
        } => {
            if !matches!(format, FormatArg::Json) {
                invalid_output("verify", "verify only writes json")
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
    };
    println!("{}", output.body);
    ExitCode::from(output.exit_code as u8)
}
