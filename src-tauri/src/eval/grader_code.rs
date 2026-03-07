//! Code-based grader: fast, deterministic checks on pipeline output.
use crate::llm::pipeline::PipelineOutput;
use super::fixture::FixtureExpected;

pub struct CodeGradeResult {
    pub score: f32,
    pub passed: Vec<String>,
    pub failed: Vec<String>,
}

/// Run all code checks against pipeline output.
/// Returns weighted score 0.0~1.0.
pub fn grade(output: &PipelineOutput, expected: &FixtureExpected) -> CodeGradeResult {
    let mut checks: Vec<(&str, f32, bool)> = Vec::new(); // (name, weight, passed)

    // Check 1: stage3 participants non-empty
    let c1 = !output.structure.participants.is_empty();
    checks.push(("stage3_participants_nonempty", 0.5, c1));

    // Check 2: stage3 required participants present
    let c2 = expected.required_participants.iter().all(|p| {
        output.structure.participants.iter().any(|op| op.contains(p.as_str()))
    });
    checks.push(("stage3_required_participants", 1.0, c2));

    // Check 3: stage4 summary contains required keywords
    let c3 = expected.summary_must_contain.iter().all(|kw| {
        output.summary.contains(kw.as_str())
    });
    checks.push(("stage4_summary_keywords", 1.0, c3));

    // Check 4: stage5 action items count >= min
    let c4 = output.action_items.len() >= expected.min_action_items;
    checks.push(("stage5_min_action_items", 1.0, c4));

    // Check 5: stage6 report non-empty
    let c5 = !output.report.trim().is_empty();
    checks.push(("stage6_report_nonempty", 0.5, c5));

    // Check 6: clean transcript non-empty (stage1)
    let c6 = !output.clean_transcript.trim().is_empty();
    checks.push(("stage1_clean_nonempty", 0.5, c6));

    // Compute weighted score
    let total_weight: f32 = checks.iter().map(|(_, w, _)| w).sum();
    let passed_weight: f32 = checks.iter().filter(|(_, _, p)| *p).map(|(_, w, _)| w).sum();
    let score = if total_weight > 0.0 { passed_weight / total_weight } else { 0.0 };

    let passed = checks.iter().filter(|(_, _, p)| *p).map(|(n, _, _)| n.to_string()).collect();
    let failed = checks.iter().filter(|(_, _, p)| !p).map(|(n, _, _)| n.to_string()).collect();

    CodeGradeResult { score, passed, failed }
}
