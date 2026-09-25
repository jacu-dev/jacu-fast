use clap::{Parser, Subcommand, ValueEnum};
use jacu_fast::{
    capabilities, clean, invalid_output, prepare, report, verify_hook, verify_retry, Checkpoint,
    HookRequest, PrepareRequest, ReportFormat, ReportRequest, VerifyRequest,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "jacu",
    version,
    about = "Prepare, verify, and report one coding task using observed evidence."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
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
        #[arg(long,value_enum,default_value_t=Format::Json)]
        format: Format,
    },
    Verify {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        session: String,
        #[arg(long,value_enum,default_value_t=Point::Delivery)]
        checkpoint: Point,
        /// Retry after a diagnosed external recovery, even with identical source inputs.
        #[arg(long)]
        retry: bool,
        #[arg(long)]
        hook: Option<String>,
        #[arg(long,value_enum,default_value_t=Format::Json)]
        format: Format,
    },
    Report {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        session: String,
        #[arg(long,value_enum,default_value_t=Format::Json)]
        format: Format,
    },
    /// Remove only expired Jacu-owned diagnostics, never worktrees or build caches.
    Clean {
        #[arg(long)]
        repo: PathBuf,
    },
    Capabilities,
}
#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Json,
    Markdown,
}
#[derive(Clone, Copy, ValueEnum)]
enum Point {
    Iteration,
    Delivery,
}
fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = e.print();
                return ExitCode::SUCCESS;
            }
            let o = invalid_output("cli", &e.to_string());
            println!("{}", o.body);
            return ExitCode::from(4);
        }
    };
    let out = match cli.command {
        Commands::Prepare {
            repo,
            request_file,
            audit,
            contract_file,
            session,
            format,
        } => {
            if matches!(format, Format::Json) {
                prepare(PrepareRequest {
                    repo,
                    request_file,
                    audit,
                    contract_file,
                    session,
                })
            } else {
                invalid_output("prepare", "prepare only writes json")
            }
        }
        Commands::Verify {
            repo,
            session,
            checkpoint,
            retry,
            hook,
            format,
        } => {
            if !matches!(format, Format::Json) {
                invalid_output("verify", "verify only writes json")
            } else if hook.is_some() {
                verify_hook(HookRequest { repo, session })
            } else {
                verify_retry(
                    VerifyRequest {
                        repo,
                        session,
                        checkpoint: match checkpoint {
                            Point::Iteration => Checkpoint::Iteration,
                            Point::Delivery => Checkpoint::Delivery,
                        },
                    },
                    retry,
                )
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
                Format::Json => ReportFormat::Json,
                Format::Markdown => ReportFormat::Markdown,
            },
        }),
        Commands::Clean { repo } => clean(repo),
        Commands::Capabilities => capabilities(),
    };
    println!("{}", out.body);
    ExitCode::from(out.exit_code as u8)
}
