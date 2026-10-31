use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use shirolyzer::{analyze, audit, check, compare, compute, context, render, xkb};

#[derive(Parser)]
#[command(name = "shirolyzer", version, about = "Layout analyzer and generator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a layout with the calibrated pattern model
    Analyze(AnalyzeArgs),
    /// Generate a layout with the calibrated pattern model
    Compute(ComputeArgs),
    /// Render a 30-character layout as a staggered ASCII keyboard
    Render(RenderArgs),
    /// Generate an XKB symbols file for an existing layout
    Xkb(XkbArgs),
    /// Score reference layouts against a corpus
    Compare(CompareArgs),
    /// Questionnaire and ruling audit for the pattern model
    Audit(AuditArgs),
    /// Verify reference scores and audit rulings against the stored baseline
    #[command(name = "check-baseline")]
    CheckBaseline(CheckArgs),
}

#[derive(Args)]
struct AnalyzeArgs {
    /// 30-character layout to analyze
    layout: Option<String>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    layout_flag: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long = "primary-corpus", action = clap::ArgAction::Append)]
    primary_corpus: Vec<String>,
    #[arg(long = "secondary-corpus", action = clap::ArgAction::Append)]
    secondary_corpus: Vec<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long = "out-json")]
    out_json: Option<PathBuf>,
}

#[derive(Args)]
struct ComputeArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long = "seed-layout", alias = "layout", action = clap::ArgAction::Append)]
    seed_layout: Vec<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long = "primary-corpus", action = clap::ArgAction::Append)]
    primary_corpus: Vec<String>,
    #[arg(long = "secondary-corpus", action = clap::ArgAction::Append)]
    secondary_corpus: Vec<String>,
    #[arg(long)]
    references: Option<PathBuf>,
    #[arg(long = "primary-sacrifice")]
    primary_sacrifice: Option<f64>,
    #[arg(long, default_value_t = 28)]
    runs: usize,
    #[arg(long, default_value_t = 200_000)]
    iters: usize,
    #[arg(long, default_value_t = 8)]
    ils: usize,
    #[arg(long = "valid-tolerance", default_value_t = 0.002)]
    valid_tolerance: f64,
    #[arg(long = "secondary-tolerance", default_value_t = 0.005)]
    secondary_tolerance: f64,
    #[arg(long = "min-layout-distance", default_value_t = 4)]
    min_layout_distance: usize,
    #[arg(long = "max-valid", default_value_t = 8)]
    max_valid: usize,
    #[arg(long = "polish-rounds", default_value_t = 0)]
    polish_rounds: usize,
    #[arg(long = "polish-candidates", default_value_t = 4)]
    polish_candidates: usize,
    #[arg(long = "append-references")]
    append_references: Option<PathBuf>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long = "time-budget")]
    time_budget: Option<String>,
    #[arg(long, value_enum, default_value = "cpu")]
    device: compute::Device,
    #[arg(long)]
    xkb: bool,
    #[arg(long = "xkb-name")]
    xkb_name: Option<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long = "out-json")]
    out_json: Option<PathBuf>,
}

#[derive(Args)]
struct RenderArgs {
    layout: String,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    plain: bool,
    #[arg(long)]
    both: bool,
}

#[derive(Args)]
struct XkbArgs {
    #[arg(long)]
    layout: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    install: bool,
}

#[derive(Args)]
struct CompareArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long = "primary-corpus")]
    primary_corpus: Option<String>,
    #[arg(long)]
    refs: Option<PathBuf>,
}

#[derive(Args)]
struct AuditArgs {
    /// Legacy model configuration name
    config_name: Option<String>,
    #[arg(long = "model-config")]
    model_config: Option<String>,
    #[arg(long = "config")]
    config_file: Option<PathBuf>,
    #[arg(long = "primary-corpus", action = clap::ArgAction::Append)]
    primary_corpus: Vec<String>,
    #[arg(long = "secondary-corpus", action = clap::ArgAction::Append)]
    secondary_corpus: Vec<String>,
    #[arg(long)]
    rulings: Option<PathBuf>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long = "out-json")]
    out_json: Option<PathBuf>,
}

#[derive(Args)]
struct CheckArgs {
    #[arg(long)]
    baseline: Option<PathBuf>,
    #[arg(long)]
    corpus: Option<PathBuf>,
    #[arg(long, default_value_t = 1e-9)]
    tolerance: f64,
    #[arg(long)]
    write: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze(args) => {
            let layout = args.layout.or(args.layout_flag);
            analyze::run(analyze::AnalyzeArgs {
                layout: layout.as_deref(),
                config: args.config.as_deref(),
                name: args.name.as_deref(),
                primary_corpus: &args.primary_corpus,
                secondary_corpus: &args.secondary_corpus,
                out: args.out.as_deref(),
                out_json: args.out_json.as_deref(),
            })
        }
        Command::Compute(args) => {
            let time_budget = match args.time_budget.as_deref() {
                Some(text) => Some(compute::parse_budget(text)?),
                None => None,
            };
            compute::run(compute::ComputeArgs {
                config: args.config.as_deref(),
                seed_layout: args.seed_layout,
                name: args.name,
                primary_corpus: args.primary_corpus,
                secondary_corpus: args.secondary_corpus,
                references: args.references,
                primary_sacrifice: args.primary_sacrifice,
                runs: args.runs,
                iters: args.iters,
                ils: args.ils,
                valid_tolerance: args.valid_tolerance,
                secondary_tolerance: args.secondary_tolerance,
                min_layout_distance: args.min_layout_distance,
                max_valid: args.max_valid,
                polish_rounds: args.polish_rounds,
                polish_candidates: args.polish_candidates,
                append_references: args.append_references,
                seed: args.seed,
                time_budget,
                device: args.device,
                xkb: args.xkb,
                xkb_name: args.xkb_name,
                out: args.out,
                out_json: args.out_json,
            })
        }
        Command::Render(args) => {
            let text = if args.plain {
                render::render(&args.layout, None).map_err(anyhow::Error::msg)?
            } else {
                let ctx = context::Context::load(args.config.as_deref())?;
                let annotated = render::render(&args.layout, Some(&ctx.cfg.finger_tokens))
                    .map_err(anyhow::Error::msg)?;
                if args.both {
                    format!(
                        "{}\n\n{}",
                        annotated,
                        render::render(&args.layout, None).map_err(anyhow::Error::msg)?
                    )
                } else {
                    annotated
                }
            };
            println!("{}", text);
            Ok(())
        }
        Command::Xkb(args) => {
            let name = args.name.clone().unwrap_or(xkb::safe_name(&args.layout)?);
            let path = args
                .out
                .clone()
                .unwrap_or_else(|| shirolyzer::paths::xkb_dir().join(&name));
            xkb::write(&path, &args.layout, &name)?;
            println!("wrote {}", path.display());
            if args.install {
                let script = shirolyzer::paths::xkb_dir().join("install.sh");
                let status = std::process::Command::new(script)
                    .arg(&path)
                    .arg(&name)
                    .status()?;
                if !status.success() {
                    return Err(anyhow::anyhow!("xkb install script failed"));
                }
            }
            Ok(())
        }
        Command::Compare(args) => compare::run(
            args.config.as_deref(),
            args.primary_corpus.as_deref(),
            args.refs.as_deref(),
        ),
        Command::Audit(args) => audit::run(audit::AuditArgs {
            config_name: args.model_config.as_deref().or(args.config_name.as_deref()),
            config_file: args.config_file.as_deref(),
            primary_corpus: &args.primary_corpus,
            secondary_corpus: &args.secondary_corpus,
            rulings: args.rulings.as_deref(),
            out: args.out.as_deref(),
            out_json: args.out_json.as_deref(),
        }),
        Command::CheckBaseline(args) => check::run(check::CheckArgs {
            baseline: args.baseline.as_deref(),
            corpus: args.corpus.as_deref(),
            tolerance: args.tolerance,
            write: args.write,
        }),
    }
}
