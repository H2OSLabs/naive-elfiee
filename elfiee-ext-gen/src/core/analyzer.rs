/// Test output analyzer for intelligent guidance.
///
/// Following generator-dev-plan.md Phase 2.2
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Data Structures (as per dev-plan.md line 509-537)
// ============================================================================

/// Test analyzer for parsing cargo test output.
pub struct TestAnalyzer {
    error_patterns: Vec<ErrorPattern>,
    dependency_patterns: Vec<DependencyPattern>,
}

/// Information about a test failure.
#[derive(Debug, Clone)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub file_location: Option<FileLocation>,
    pub matched_pattern: Option<ErrorPattern>,
}

/// File location extracted from error message.
#[derive(Debug, Clone)]
pub struct FileLocation {
    pub file: String,
    pub line: usize, // usize per dev-plan line 520
}

/// Error pattern loaded from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub category: String,
    pub hint: String,
}

/// Test dependency pattern for automatic dependency inference.
#[derive(Debug, Clone, Deserialize)]
pub struct DependencyPattern {
    pub test_pattern: String,
    pub depends_on_pattern: Option<String>,
    pub reason: String,
}

/// Analysis report summarizing test results.
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    pub total_tests: usize,
    pub passing: usize,
    pub failing: Vec<TestFailure>,
    pub critical_path: Vec<TestFailure>,
    pub estimated_time: Duration,
}

impl TestAnalyzer {
    /// Create a new TestAnalyzer instance.
    ///
    /// Loads error patterns from rules/error_patterns.yaml.
    ///
    /// # Examples
    /// ```no_run
    /// use elfiee_ext_gen::core::analyzer::TestAnalyzer;
    ///
    /// let analyzer = TestAnalyzer::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, String> {
        // Load error patterns from YAML file
        let yaml_content = Self::load_yaml("rules/error_patterns.yaml")?;

        #[derive(Deserialize)]
        struct PatternFile {
            patterns: Vec<ErrorPattern>,
        }

        let pattern_file: PatternFile = serde_yaml::from_str(&yaml_content)
            .map_err(|e| format!("Failed to parse error_patterns.yaml: {}", e))?;

        // Load dependency patterns (optional file)
        let dependency_patterns = match Self::load_yaml("rules/test_dependencies.yaml") {
            Ok(content) => {
                #[derive(Deserialize)]
                struct DependencyFile {
                    dependency_patterns: Vec<DependencyPattern>,
                }
                let dep_file: DependencyFile = serde_yaml::from_str(&content)
                    .map_err(|e| format!("Failed to parse test_dependencies.yaml: {}", e))?;
                dep_file.dependency_patterns
            }
            Err(_) => Vec::new(),
        };

        Ok(Self {
            error_patterns: pattern_file.patterns,
            dependency_patterns,
        })
    }

    /// Analyze test output and generate report.
    ///
    /// # Arguments
    /// * `test_output` - Output from `cargo test`
    ///
    /// # Returns
    /// * `AnalysisReport` with parsed failures and suggestions
    pub fn analyze(&self, test_output: &str) -> AnalysisReport {
        // Extract total number of tests
        let total_tests = if let Some(cap) = Regex::new(r"running (\d+) tests?")
            .unwrap()
            .captures(test_output)
        {
            cap[1].parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        // Parse all failures
        let failing = self.parse_failures(test_output);
        let passing = total_tests.saturating_sub(failing.len());

        // Compute critical path
        let critical_path = self.compute_critical_path(&failing);

        // Estimate time: 15 minutes per failing test
        let estimated_time = Duration::from_secs(failing.len() as u64 * 15 * 60);

        AnalysisReport {
            total_tests,
            passing,
            failing,
            critical_path,
            estimated_time,
        }
    }

    /// Parse failures from test output.
    ///
    /// # Arguments
    /// * `output` - Raw test output
    ///
    /// # Returns
    /// * Vec of TestFailure with extracted information
    fn parse_failures(&self, output: &str) -> Vec<TestFailure> {
        let mut failures = Vec::new();

        // Regex to match failed test lines: "test <name> ... FAILED"
        let test_fail_re = Regex::new(r"test\s+([\w:]+)\s+\.\.\.\s+FAILED").unwrap();

        // Extract all failed test names
        let failed_test_names: Vec<String> = test_fail_re
            .captures_iter(output)
            .map(|cap| cap[1].to_string())
            .collect();

        // For each failed test, try to extract detailed error info
        for test_name in failed_test_names {
            // Look for the detailed error section: "---- <test_name> stdout ----"
            let raw_error = {
                let pattern = format!(
                    r"---- {}\s+(stdout|stderr) ----\n",
                    regex::escape(&test_name)
                );

                if let Ok(re) = Regex::new(&pattern) {
                    if let Some(mat) = re.find(output) {
                        let section_start = mat.end();
                        let section_end = output[section_start..]
                            .find("\n---- ")
                            .map(|pos| section_start + pos)
                            .unwrap_or(output.len());

                        output[section_start..section_end].trim().to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            };

            let raw_error = if raw_error.is_empty() {
                Self::extract_panic_summary(output, &test_name).unwrap_or_else(String::new)
            } else {
                raw_error
            };

            let error_message = Self::strip_ansi_codes(&raw_error);
            let file_location = self.extract_location(&error_message);
            let matched_pattern = self.match_pattern(&error_message);

            failures.push(TestFailure {
                test_name,
                error_message,
                file_location,
                matched_pattern,
            });
        }

        failures
    }

    /// Match error message against known patterns.
    ///
    /// # Arguments
    /// * `error_msg` - Error message from test failure
    ///
    /// # Returns
    /// * Some(ErrorPattern) if matched, None otherwise
    fn match_pattern(&self, error_msg: &str) -> Option<ErrorPattern> {
        for pattern in &self.error_patterns {
            if let Ok(re) = Regex::new(&pattern.pattern) {
                if re.is_match(error_msg) {
                    return Some(pattern.clone());
                }
            }
        }
        None
    }

    /// Extract file location from error message.
    ///
    /// # Arguments
    /// * `error_msg` - Error message containing file:line info
    ///
    /// # Returns
    /// * Some(FileLocation) if found, None otherwise
    fn extract_location(&self, error_msg: &str) -> Option<FileLocation> {
        // Regex to match file:line:column pattern
        // Example: "src/extensions/todo/todo_add.rs:15:5"
        let location_re = Regex::new(r"([\w/]+\.rs):(\d+):\d+").unwrap();

        if let Some(cap) = location_re.captures(error_msg) {
            let file = cap[1].to_string();
            let line = cap[2].parse::<usize>().ok()?;

            Some(FileLocation { file, line })
        } else {
            None
        }
    }

    /// Compute critical path (tests to fix first) using pattern-based dependency rules.
    ///
    /// # Arguments
    /// * `failures` - All test failures
    ///
    /// # Returns
    /// * Vec of failures ordered by priority (tests with no failing dependencies first)
    fn compute_critical_path(&self, failures: &[TestFailure]) -> Vec<TestFailure> {
        let failing_names: HashSet<_> = failures.iter().map(|f| f.test_name.clone()).collect();
        let mut roots = Vec::new();

        for failure in failures {
            // Find dependency patterns that match this test
            let dependencies = self.get_dependencies_for_test(&failure.test_name, &failing_names);

            // If all dependencies are passing (or no dependencies), this is a root test
            if dependencies.is_empty() {
                roots.push(failure.clone());
            }
        }

        // Sort by priority: tests with matched error patterns first
        roots.sort_by_key(|f| match &f.matched_pattern {
            Some(pattern) if pattern.category == "payload_field_missing" => 0,
            Some(pattern) if pattern.category == "todo_marker" => 1,
            Some(_) => 2,
            None => 3,
        });

        roots
    }

    /// Get failing dependencies for a test based on pattern matching.
    ///
    /// # Arguments
    /// * `test_name` - Name of the test to check
    /// * `failing_names` - Set of all failing test names
    ///
    /// # Returns
    /// * Vec of failing dependency test names
    fn get_dependencies_for_test(
        &self,
        test_name: &str,
        failing_names: &HashSet<String>,
    ) -> Vec<String> {
        let mut dependencies = Vec::new();

        for pattern in &self.dependency_patterns {
            // Check if this test matches the pattern
            if let Ok(test_regex) = Regex::new(&pattern.test_pattern) {
                if test_regex.is_match(test_name) {
                    // If pattern has no dependency, skip
                    if pattern.depends_on_pattern.is_none() {
                        continue;
                    }

                    // Find all failing tests that match the dependency pattern
                    let dep_pattern = pattern.depends_on_pattern.as_ref().unwrap();
                    if let Ok(dep_regex) = Regex::new(dep_pattern) {
                        for failing_test in failing_names {
                            if dep_regex.is_match(failing_test) {
                                dependencies.push(failing_test.clone());
                            }
                        }
                    }
                }
            }
        }

        dependencies
    }

    fn load_yaml(relative_path: &str) -> Result<String, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join(relative_path);
            if path.exists() {
                return std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {}", path.display(), e));
            }
        }

        let manifest_path = manifest_dir.join(relative_path);
        if manifest_path.exists() {
            return std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("Failed to read {}: {}", manifest_path.display(), e));
        }

        Err(format!("Failed to locate {}", relative_path))
    }

    fn strip_ansi_codes(text: &str) -> String {
        let ansi_re = Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
        ansi_re.replace_all(text, "").to_string()
    }

    fn extract_panic_summary(output: &str, test_name: &str) -> Option<String> {
        let pattern = format!(
            r"thread '{}'.*panicked at [^\n]+\n([^\n]+)",
            regex::escape(test_name)
        );

        let re = Regex::new(&pattern).ok()?;
        let caps = re.captures(output)?;
        Some(caps.get(1)?.as_str().trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_lock;

    fn with_analyzer<F, R>(f: F) -> R
    where
        F: FnOnce(TestAnalyzer) -> R,
    {
        let _guard = test_lock().lock().unwrap();
        let analyzer = TestAnalyzer::new().unwrap();
        f(analyzer)
    }

    // Sample test output for testing (from dev-plan.md line 567-579)
    const SAMPLE_TEST_OUTPUT: &str = r#"
running 5 tests
test todo::tests::test_add_item_basic ... FAILED
test todo::tests::test_toggle_item ... FAILED
test todo::tests::test_owner_auth ... ok
test todo::tests::test_full_workflow ... FAILED
test todo::tests::test_validation ... ok

failures:

---- todo::tests::test_add_item_basic stdout ----
thread 'todo::tests::test_add_item_basic' panicked at 'not yet implemented: Deserialize TodoAddPayload from cmd.payload', src/extensions/todo/todo_add.rs:15:5
"#;

    // ========================================
    // Test: parse_failures()
    // (Following dev-plan.md line 582-588)
    // ========================================

    #[test]
    fn test_parse_failures() {
        with_analyzer(|analyzer| {
            let failures = analyzer.parse_failures(SAMPLE_TEST_OUTPUT);

            assert_eq!(failures.len(), 3);
            assert!(failures
                .iter()
                .any(|f| f.test_name.contains("test_add_item_basic")));
        });
    }

    // ========================================
    // Test: extract_location()
    // (Following dev-plan.md line 590-598)
    // ========================================

    #[test]
    fn test_extract_location() {
        with_analyzer(|analyzer| {
            let error = "panicked at 'message', src/extensions/todo/todo_add.rs:15:5";

            let location = analyzer.extract_location(error).unwrap();
            assert_eq!(location.file, "src/extensions/todo/todo_add.rs");
            assert_eq!(location.line, 15);
        });
    }

    // ========================================
    // Test: match_pattern()
    // (Following dev-plan.md line 600-608)
    // ========================================

    #[test]
    fn test_match_pattern_todo() {
        with_analyzer(|analyzer| {
            let error = "not yet implemented: Deserialize payload";

            let pattern = analyzer.match_pattern(error).unwrap();
            assert_eq!(pattern.category, "todo_marker");
            assert!(pattern.hint.contains("implement"));
        });
    }

    // ========================================
    // Test: analyze()
    // (Following dev-plan.md line 610-618)
    // ========================================

    #[test]
    fn test_analyze_report() {
        with_analyzer(|analyzer| {
            let report = analyzer.analyze(SAMPLE_TEST_OUTPUT);

            assert_eq!(report.total_tests, 5);
            assert_eq!(report.passing, 2);
            assert_eq!(report.failing.len(), 3);
        });
    }

    // ========================================
    // Test: compute_critical_path()
    // (Following dev-plan.md line 620-646)
    // ========================================

    #[test]
    fn test_compute_critical_path() {
        with_analyzer(|analyzer| {
            let failures = vec![
                TestFailure {
                    test_name: "test_add_item".to_string(),
                    error_message: "todo!()".to_string(),
                    file_location: None,
                    matched_pattern: Some(ErrorPattern {
                        pattern: "todo".to_string(),
                        category: "todo_marker".to_string(),
                        hint: "Implement".to_string(),
                    }),
                },
                TestFailure {
                    test_name: "test_workflow".to_string(),
                    error_message: "depends on add_item".to_string(),
                    file_location: None,
                    matched_pattern: None,
                },
            ];

            let critical = analyzer.compute_critical_path(&failures);

            // test_add_item 应该在 critical path 中（有 matched_pattern）
            assert!(critical.iter().any(|f| f.test_name == "test_add_item"));
        });
    }

    #[test]
    fn test_compute_critical_path_respects_dependencies() {
        with_analyzer(|analyzer| {
            let failures = vec![
                TestFailure {
                    test_name: "test_full_workflow_add_toggle_remove".to_string(),
                    error_message: "workflow failed".to_string(),
                    file_location: None,
                    matched_pattern: None,
                },
                TestFailure {
                    test_name: "test_add_item_basic".to_string(),
                    error_message: "todo!()".to_string(),
                    file_location: None,
                    matched_pattern: Some(ErrorPattern {
                        pattern: "todo".to_string(),
                        category: "todo_marker".to_string(),
                        hint: "Implement".to_string(),
                    }),
                },
            ];

            let critical = analyzer.compute_critical_path(&failures);

            assert_eq!(
                critical.len(),
                1,
                "Only root failures without unresolved prerequisites should be highlighted"
            );
            assert_eq!(critical[0].test_name, "test_add_item_basic");
        });
    }
}
