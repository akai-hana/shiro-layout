use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use shirolyzer::{analyze, audit, check, compare, compute, context, render, worst, xkb};

#[derive(Parser)]
#[command(name = "shirolyzer", version, about = "Layout analyzer and generator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Analyze a layout with the calibrated pattern model
    Analyze(AnalyzeArgs),
    /// List the highest-cost bigrams and trigrams for a layout
    Worst(WorstArgs),
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
struct WorstArgs {
    /// 30-character layout to inspect (defaults to the latest compute result)
    layout: Option<String>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long = "primary-corpus", action = clap::ArgAction::Append)]
    primary_corpus: Vec<String>,
    /// Show only the first N n-grams per kind; 0 shows all
    #[arg(long, default_value_t = 0)]
    limit: usize,
    #[arg(long)]
    out: Option<PathBuf>,
}

fn parse_polish_rounds(v: &str) -> Result<usize, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "unlimited" | "inf" | "infinite" | "infinity" | "max" => Ok(usize::MAX),
        n => n.parse::<usize>().map_err(|_| {
            format!(
                "expected a non-negative integer or \"unlimited\", got {:?}",
                v
            )
        }),
    }
}

#[derive(Args)]
struct ComputeArgs {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long = "seed-layout", alias = "layout", action = clap::ArgAction::Append)]
    seed_layout: Vec<String>,
    /// Also start from the named layout in the references file (repeatable)
    #[arg(long = "seed-reference", action = clap::ArgAction::Append)]
    seed_reference: Vec<String>,
    /// Also start from every (allowed) layout in the references file
    #[arg(long = "seed-all-references")]
    seed_all_references: bool,
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
    /// Number of diverse top layouts advanced to ILS
    #[arg(long = "ils-candidates", default_value_t = 4)]
    ils_candidates: usize,
    /// Perturbation rounds per candidate in the secondary phase (default: same as --ils; 0 only scores them)
    #[arg(long = "secondary-rounds")]
    secondary_rounds: Option<usize>,
    /// Random starts run per batch; batches are sequential so early stopping can occur between them
    #[arg(long = "batch-size", default_value_t = 8)]
    batch_size: usize,
    /// Stop after the best letter placement is found by this many random starts (0 disables)
    #[arg(long = "stop-hits", default_value_t = 0)]
    stop_hits: usize,
    /// Stop after this many consecutive batches without a primary improvement (0 disables)
    #[arg(long = "stop-stale-batches", default_value_t = 0)]
    stop_stale_batches: usize,
    #[arg(long = "valid-tolerance", default_value_t = 0.002)]
    valid_tolerance: f64,
    #[arg(long = "secondary-tolerance", default_value_t = 0.005)]
    secondary_tolerance: f64,
    #[arg(long = "min-layout-distance", default_value_t = 4)]
    min_layout_distance: usize,
    #[arg(long = "max-valid", default_value_t = 8)]
    max_valid: usize,
    /// Max polish rounds per candidate: a number, "unlimited" (default), or 0 to
    /// disable.  Polishing always stops early once a round finds no improvement.
    #[arg(long = "polish-rounds", default_value = "unlimited", value_parser = parse_polish_rounds)]
    polish_rounds: usize,
    #[arg(long = "polish-candidates", default_value_t = 4)]
    polish_candidates: usize,
    #[arg(long = "append-references")]
    append_references: Option<PathBuf>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long = "time-budget")]
    time_budget: Option<String>,
    /// List every distinct layout from the anneal stage, with its layout string
    #[arg(long, short = 'v')]
    verbose: bool,
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
    #[arg(long = "primary-corpus", conflicts_with = "corpus_json")]
    primary_corpus: Option<String>,
    /// Read an external letters/bigrams/trigrams JSON bundle (comparison only)
    #[arg(long = "corpus-json")]
    corpus_json: Option<PathBuf>,
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
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    rulings: Option<PathBuf>,
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
        Command::Worst(args) => worst::run(worst::WorstArgs {
            layout: args.layout.as_deref(),
            config: args.config.as_deref(),
            primary_corpus: &args.primary_corpus,
            limit: args.limit,
            out: args.out.as_deref(),
        }),
        Command::Compute(args) => {
            let time_budget = match args.time_budget.as_deref() {
                Some(text) => Some(compute::parse_budget(text)?),
                None => None,
            };
            compute::run(compute::ComputeArgs {
                config: args.config.as_deref(),
                seed_layout: args.seed_layout,
                seed_reference: args.seed_reference,
                seed_all_references: args.seed_all_references,
                name: args.name,
                primary_corpus: args.primary_corpus,
                secondary_corpus: args.secondary_corpus,
                references: args.references,
                primary_sacrifice: args.primary_sacrifice,
                runs: args.runs,
                iters: args.iters,
                ils: args.ils,
                ils_candidates: args.ils_candidates,
                secondary_rounds: args.secondary_rounds,
                batch_size: args.batch_size,
                stop_hits: args.stop_hits,
                stop_stale_batches: args.stop_stale_batches,
                valid_tolerance: args.valid_tolerance,
                secondary_tolerance: args.secondary_tolerance,
                min_layout_distance: args.min_layout_distance,
                max_valid: args.max_valid,
                polish_rounds: args.polish_rounds,
                polish_candidates: args.polish_candidates,
                append_references: args.append_references,
                seed: args.seed,
                time_budget,
                verbose: args.verbose,
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
            args.corpus_json.as_deref(),
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
            config: args.config.as_deref(),
            rulings: args.rulings.as_deref(),
            tolerance: args.tolerance,
            write: args.write,
        }),
    }
}
