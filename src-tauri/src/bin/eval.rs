//! eval binary: run LLM pipeline against fixtures and produce quality report.
//!
//! Usage:
//!   cargo run --bin eval
//!   cargo run --bin eval -- --fixture tech_review
//!   cargo run --bin eval -- --no-llm-judge
//!   cargo run --bin eval -- --config path/to/config.json
//!   cargo run --bin eval -- --output report.md

use memo_ai_lib::{
    eval::{
        fixture::{EvalResult, Fixture},
        grader_code,
        grader_llm,
        reporter,
    },
    llm::client::{LlmClient, LlmConfig},
    llm::pipeline::Pipeline,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let cli = parse_args(&args);

    println!("memo-ai eval — loading fixtures...");

    let workspace_root = find_workspace_root().expect("Cannot find workspace root");
    let fixtures_dir = workspace_root.join("evals/fixtures");
    let rubrics_dir = workspace_root.join("evals/rubrics");
    let prompts_dir = workspace_root.join("prompts");

    let mut fixtures = Fixture::load_all(&fixtures_dir)
        .expect("Failed to load fixtures");

    if let Some(ref filter) = cli.fixture_filter {
        fixtures.retain(|f| f.meta.id.contains(filter.as_str()));
        println!("Filtered to {} fixture(s) matching '{}'", fixtures.len(), filter);
    }

    if fixtures.is_empty() {
        eprintln!("No fixtures found in {:?}", fixtures_dir);
        std::process::exit(1);
    }

    let config = load_llm_config(&cli.config_path, &workspace_root);
    let client = config.build_client();
    println!("Using LLM: {} @ {}", config.model, config.base_url);

    let mut results: Vec<EvalResult> = Vec::new();
    for fixture in &fixtures {
        println!("\nRunning: {} ({})", fixture.meta.id, fixture.meta.scene);
        let result = run_fixture(
            fixture,
            client.as_ref(),
            &prompts_dir,
            &rubrics_dir,
            cli.no_llm_judge,
        );
        println!("  code={:.2} llm={} status={}",
            result.code_score,
            result.llm_score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "—".into()),
            result.status()
        );
        results.push(result);
    }

    let report = reporter::generate(&results);

    match &cli.output_path {
        Some(path) => {
            std::fs::write(path, &report).expect("Failed to write report");
            println!("\nReport saved to {:?}", path);
        }
        None => {
            println!("\n{}", report);
        }
    }

    let has_fail = results.iter().any(|r| r.status() == "FAIL");
    if has_fail {
        std::process::exit(1);
    }
}

fn run_fixture(
    fixture: &Fixture,
    client: &dyn LlmClient,
    prompts_dir: &Path,
    rubrics_dir: &Path,
    no_llm_judge: bool,
) -> EvalResult {
    let start = Instant::now();
    let pipeline = Pipeline::new(client, prompts_dir);

    let output = match pipeline.run(&fixture.input.raw_transcript, false) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("  Pipeline error: {}", e);
            return EvalResult {
                fixture_id: fixture.meta.id.clone(),
                scene: fixture.meta.scene.clone(),
                difficulty: fixture.meta.difficulty.clone(),
                code_score: 0.0,
                llm_score: None,
                llm_reason: Some(format!("Pipeline error: {}", e)),
                passed_checks: vec![],
                failed_checks: vec!["pipeline_run".to_string()],
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    let code_result = grader_code::grade(&output, &fixture.expected);

    let (llm_score, llm_reason) = if !no_llm_judge
        && code_result.score >= 0.6
        && fixture.expected.golden_summary.as_deref().map(|s| !s.is_empty()).unwrap_or(false)
    {
        match grader_llm::grade(
            client,
            rubrics_dir,
            fixture.expected.golden_summary.as_deref().unwrap(),
            &output.summary,
        ) {
            Ok(r) => (Some(r.score), Some(r.reason)),
            Err(e) => {
                eprintln!("  LLM judge error: {}", e);
                (None, Some(format!("Judge error: {}", e)))
            }
        }
    } else {
        (None, None)
    };

    EvalResult {
        fixture_id: fixture.meta.id.clone(),
        scene: fixture.meta.scene.clone(),
        difficulty: fixture.meta.difficulty.clone(),
        code_score: code_result.score,
        llm_score,
        llm_reason,
        passed_checks: code_result.passed,
        failed_checks: code_result.failed,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

struct CliArgs {
    fixture_filter: Option<String>,
    no_llm_judge: bool,
    config_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> CliArgs {
    let mut cli = CliArgs {
        fixture_filter: None,
        no_llm_judge: false,
        config_path: None,
        output_path: None,
    };
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--fixture" => { i += 1; cli.fixture_filter = args.get(i).cloned(); }
            "--no-llm-judge" => { cli.no_llm_judge = true; }
            "--config" => { i += 1; cli.config_path = args.get(i).map(PathBuf::from); }
            "--output" => { i += 1; cli.output_path = args.get(i).map(PathBuf::from); }
            _ => {}
        }
        i += 1;
    }
    cli
}

fn load_llm_config(config_path: &Option<PathBuf>, workspace_root: &Path) -> LlmConfig {
    if let Some(path) = config_path {
        if let Ok(s) = std::fs::read_to_string(path) {
            if let Ok(c) = serde_json::from_str::<LlmConfig>(&s) {
                return c;
            }
        }
    }
    let settings_path = workspace_root.join("eval-config.json");
    if let Ok(s) = std::fs::read_to_string(&settings_path) {
        if let Ok(c) = serde_json::from_str::<LlmConfig>(&s) {
            return c;
        }
    }
    LlmConfig {
        provider: "ollama".into(),
        base_url: "http://localhost:11434".into(),
        model: "llama3".into(),
        api_key: None,
    }
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..10 {
        if dir.join("evals").exists() {
            return Some(dir);
        }
        if !dir.pop() { break; }
    }
    std::env::current_dir().ok().and_then(|d| d.parent().map(PathBuf::from))
}
