use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use rand::SeedableRng;
use shirolyzer::evaluate::{dead_of, flat_share_from_config, perm_of, Evaluator};
use shirolyzer::search::{anneal, delta_swap_cost, quench, quench_generic, DeltaTables};
use shirolyzer::{conditions, config, corpus, gates, geometry, scorer};

fn main() {
    let cfg = config::load(None).expect("config");
    let geom = geometry::Geometry::new();
    let model = config::build_model(&cfg).expect("model");
    let gates = gates::Gates::from_config(&cfg);
    let conds = conditions::Conditions::from_config(&cfg, &geom);
    let start = Instant::now();
    let tables = scorer::Tables::build(&model, &geom, &cfg, &gates, &conds);
    println!(
        "tables build: {:.3} ms",
        start.elapsed().as_secs_f64() * 1e3
    );
    let paths: Vec<PathBuf> = cfg
        .primary_corpus
        .iter()
        .map(|e| cfg.corpus_path(e))
        .collect();
    let corpus = corpus::load_many(&paths).expect("corpus");
    let dead = dead_of(&cfg.current_layout);
    let perm = perm_of(&cfg.current_layout, &dead).expect("perm");
    let (w_balance, w_hand) = cfg.balance_weights();
    let ev = Evaluator::new(
        &corpus,
        &tables,
        &model,
        &dead,
        flat_share_from_config(&cfg),
        w_balance,
        w_hand,
    )
    .expect("evaluator");

    let start = Instant::now();
    let delta_tables = DeltaTables::build(&ev);
    let build = start.elapsed();
    println!("delta tables build: {:.3} ms", build.as_secs_f64() * 1e3);

    let n = 2_000;
    let start = Instant::now();
    for _ in 0..n {
        black_box(ev.score(black_box(&perm)));
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "full score: {:.2} us/op, {:.0} ops/s",
        elapsed / n as f64 * 1e6,
        n as f64 / elapsed
    );

    let reps = 200;
    let start = Instant::now();
    black_box(delta_swap_cost(&ev, &perm, reps));
    let elapsed = start.elapsed().as_secs_f64();
    let ops = reps * 325;
    println!(
        "delta swap: {:.0} ns/op, {:.0} ops/s",
        elapsed / ops as f64 * 1e9,
        ops as f64 / elapsed
    );

    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
    let start = Instant::now();
    let (_, _) = quench(&ev, &delta_tables, &perm, ev.score(&perm), &mut rng);
    println!("quench: {:.3} ms", start.elapsed().as_secs_f64() * 1e3);

    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
    let start = Instant::now();
    let (_, _) = quench_generic(&ev, &perm, ev.score(&perm), &mut rng);
    println!(
        "full-score quench: {:.3} ms",
        start.elapsed().as_secs_f64() * 1e3
    );

    let iters = 10_000;
    let start = Instant::now();
    let (_, _) = anneal(&ev, Some(&perm), iters, 0.35, 0.002, 7);
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "anneal: {:.3} ms for {} iters ({:.0} iters/s)",
        elapsed * 1e3,
        iters,
        iters as f64 / elapsed
    );
}
