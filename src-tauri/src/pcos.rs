use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CognitiveOverview {
    pub release: String,
    pub tagline: String,
    pub readiness_status: String,
    pub privacy_mode: String,
    pub memory_score: i64,
    pub module_count: i64,
    pub enterprise_controls: i64,
    pub use_case_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CognitiveModule {
    pub id: i64,
    pub name: String,
    pub layer: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct EnterpriseControl {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub scope: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CognitiveUseCase {
    pub id: i64,
    pub audience: String,
    pub workflow: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CognitiveReleaseResult {
    pub modules_verified: i64,
    pub controls_verified: i64,
    pub use_cases_verified: i64,
    pub status: String,
}

pub fn memory_score(modules: i64, controls: i64, use_cases: i64, platform_score: i64) -> i64 {
    ((modules * 6) + (controls * 5) + (use_cases * 3) + platform_score).clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::memory_score;

    #[test]
    fn memory_score_is_capped() {
        assert_eq!(memory_score(20, 20, 20, 100), 100);
    }
}
