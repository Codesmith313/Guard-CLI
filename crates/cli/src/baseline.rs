/// Baseline support for adopting soroban-guard on existing codebases.
///
/// Baseline records findings that exist today and fails only on new findings,
/// allowing projects to adopt soroban-guard without suppressing existing issues.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FindingFingerprint {
    pub check_name: String,
    pub file_path: String,
    pub function_name: Option<String>,
    pub description: String,
}

impl FindingFingerprint {
    /// Create a fingerprint from a finding (deliberately excluding line number).
    pub fn from_finding(
        check_name: String,
        file_path: String,
        function_name: Option<String>,
        description: String,
    ) -> Self {
        Self {
            check_name,
            file_path,
            function_name,
            description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineFile {
    pub version: String,
    pub created_at: String,
    pub findings: Vec<FindingFingerprint>,
}

impl BaselineFile {
    pub fn new(findings: Vec<FindingFingerprint>) -> Self {
        Self {
            version: "1.0".to_string(),
            created_at: rfc3339_now(),
            findings,
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let baseline = serde_json::from_str(&content)?;
        Ok(baseline)
    }

    pub fn as_set(&self) -> HashSet<FindingFingerprint> {
        self.findings.iter().cloned().collect()
    }
}

/// Produce an RFC 3339 UTC timestamp without pulling in a date/time crate.
fn rfc3339_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = civil_from_unix(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

/// Convert a Unix timestamp (seconds) into a UTC civil date/time tuple.
fn civil_from_unix(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = (rem / 3_600) as u32;
    let minute = ((rem % 3_600) / 60) as u32;
    let second = (rem % 60) as u32;

    // Howard Hinnant's civil_from_days algorithm.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };

    (year, month, day, hour, minute, second)
}

/// Filter findings against baseline, returning only new findings.
pub fn filter_findings(
    current_findings: Vec<FindingFingerprint>,
    baseline: Option<&BaselineFile>,
) -> (Vec<FindingFingerprint>, usize) {
    if let Some(baseline) = baseline {
        let baseline_set = baseline.as_set();
        let total = current_findings.len();
        let new_findings: Vec<FindingFingerprint> = current_findings
            .into_iter()
            .filter(|f| !baseline_set.contains(f))
            .collect();
        let hidden_count = total - new_findings.len();
        (new_findings, hidden_count)
    } else {
        (current_findings, 0)
    }
}

/// Compare baseline against current findings to identify fixed issues.
pub fn identify_fixed_findings(
    baseline: &BaselineFile,
    current_findings: &[FindingFingerprint],
) -> Vec<FindingFingerprint> {
    let current_set: HashSet<_> = current_findings.iter().cloned().collect();
    baseline
        .findings
        .iter()
        .filter(|f| !current_set.contains(f))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_fingerprint_creation() {
        let fp = FindingFingerprint::from_finding(
            "missing-zero-address-check".to_string(),
            "src/contract.rs".to_string(),
            Some("transfer".to_string()),
            "Admin address not checked for zero".to_string(),
        );

        assert_eq!(fp.check_name, "missing-zero-address-check");
        assert_eq!(fp.file_path, "src/contract.rs");
        assert_eq!(fp.function_name, Some("transfer".to_string()));
    }

    #[test]
    fn test_filter_findings_no_baseline() {
        let findings = vec![
            FindingFingerprint::from_finding(
                "check1".to_string(),
                "file1.rs".to_string(),
                None,
                "Issue 1".to_string(),
            ),
        ];

        let (filtered, hidden) = filter_findings(findings.clone(), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(hidden, 0);
    }

    #[test]
    fn test_filter_findings_with_baseline() {
        let baseline = BaselineFile::new(vec![
            FindingFingerprint::from_finding(
                "check1".to_string(),
                "file1.rs".to_string(),
                None,
                "Issue 1".to_string(),
            ),
        ]);

        let findings = vec![
            FindingFingerprint::from_finding(
                "check1".to_string(),
                "file1.rs".to_string(),
                None,
                "Issue 1".to_string(),
            ),
            FindingFingerprint::from_finding(
                "check2".to_string(),
                "file2.rs".to_string(),
                None,
                "Issue 2".to_string(),
            ),
        ];

        let (filtered, hidden) = filter_findings(findings, Some(&baseline));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].check_name, "check2");
        assert_eq!(hidden, 1);
    }

    #[test]
    fn test_identify_fixed_findings() {
        let baseline = BaselineFile::new(vec![
            FindingFingerprint::from_finding(
                "check1".to_string(),
                "file1.rs".to_string(),
                None,
                "Issue 1".to_string(),
            ),
            FindingFingerprint::from_finding(
                "check2".to_string(),
                "file2.rs".to_string(),
                None,
                "Issue 2".to_string(),
            ),
        ]);

        let current = vec![
            FindingFingerprint::from_finding(
                "check1".to_string(),
                "file1.rs".to_string(),
                None,
                "Issue 1".to_string(),
            ),
        ];

        let fixed = identify_fixed_findings(&baseline, &current);
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[0].check_name, "check2");
    }

    #[test]
    fn test_rfc3339_now_format() {
        let ts = rfc3339_now();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
    }
}
