use super::fixture::EvalResult;
use chrono::Local;

pub fn generate(results: &[EvalResult]) -> String {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let total = results.len();
    let passed = results.iter().filter(|r| r.status() == "PASS").count();
    let avg_llm: Option<f32> = {
        let scores: Vec<f32> = results.iter().filter_map(|r| r.llm_score).collect();
        if scores.is_empty() { None } else { Some(scores.iter().sum::<f32>() / scores.len() as f32) }
    };

    let mut out = format!("# memo-ai Eval Report — {}\n\n", date);

    out.push_str("| Fixture | Scene | Code分 | LLM分 | 耗时 | 状态 |\n");
    out.push_str("|---------|-------|--------|-------|------|------|\n");
    for r in results {
        let llm_str = r.llm_score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "—".into());
        out.push_str(&format!(
            "| {} | {} | {:.2} | {} | {}ms | {} |\n",
            r.fixture_id, r.scene, r.code_score, llm_str, r.duration_ms, r.status()
        ));
    }

    let pass_pct = if total == 0 { 0.0 } else { passed as f32 / total as f32 * 100.0 };
    out.push_str(&format!("\n**总体通过率：{}/{}（{:.0}%）**\n", passed, total, pass_pct));
    if let Some(avg) = avg_llm {
        out.push_str(&format!("**平均 LLM 评分：{:.2}**\n", avg));
    }

    let failures: Vec<&EvalResult> = results.iter().filter(|r| r.status() == "FAIL" || r.status() == "WARN").collect();
    if !failures.is_empty() {
        out.push_str("\n## 未通过详情\n");
        for r in failures {
            out.push_str(&format!("\n### {} ({})\n", r.fixture_id, r.status()));
            for check in &r.failed_checks {
                out.push_str(&format!("- FAIL {}\n", check));
            }
            if let Some(ref reason) = r.llm_reason {
                out.push_str(&format!("- LLM Judge: {}\n", reason));
            }
        }
    }

    out
}
