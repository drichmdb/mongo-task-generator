//! Representation of a resmoke suite file.

use std::{collections::HashSet, str::FromStr};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml::{Error, Value};
use tracing::error;

#[derive(Serialize, Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TestRoot {
    /// The path to a file containing the list of root tests.
    Root { root: String },
    /// A list of root tests.
    Roots { roots: Vec<String> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResmokeSelector {
    /// A str or dict representing a tag matching expression that the tags of the
    /// selected tests must not match. Incompatible with 'include_tags'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_tags: Option<String>,
    /// A list of paths or glob patterns the tests must not be included in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_files: Option<Vec<String>>,
    /// A list of tags. No selected tests can have any of them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_with_any_tags: Option<HashSet<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_count_multiplier: Option<f64>,
    /// A list of tags. All selected tests must have at least one them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_with_any_tags: Option<Vec<String>>,
    /// A list of paths or glob patterns the tests must be included in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_files: Option<Vec<String>>,
    /// A str or dict representing a tag matching expression that the tags of the
    /// selected tests must match. Incompatible with 'exclude_tags'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_tags: Option<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub test_root: Option<TestRoot>,
    /// Filename of a tag file associating tests to tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct ResmokeExecutor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<Box<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Box<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixture: Option<Box<Value>>,
}

/// Configuration of a resmoke test suite.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResmokeSuiteConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_suite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub test_kind: String,
    pub selector: ResmokeSelector,
    pub executor: ResmokeExecutor,
}

impl FromStr for ResmokeSuiteConfig {
    type Err = Error;

    /// Read Resmoke suite configuration from the given string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let resmoke_suite_config: Result<Self, serde_yaml::Error> = serde_yaml::from_str(s);
        if resmoke_suite_config.is_err() {
            error!(yaml = s, "Failed to parse yaml for ResmokeSuiteConfig",);
        }
        resmoke_suite_config
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ResmokeSuiteConfig {
    /// Convert this resmoke suite configuration to a string.
    fn to_string(&self) -> String {
        serde_yaml::to_string(self).unwrap()
    }
}

impl ResmokeSuiteConfig {
    /// Create a new resmoke suite configuration based on this one but running certain tests.
    ///
    /// # Arguments
    ///
    /// * `run_tests` - When provided, the new configuration should only run these tests.
    /// * `exclude_tests` - When provided, the new configuration should exclude these tests.
    ///
    /// # Returns
    ///
    /// New resmoke configuration with a selector based on provided parameters.
    pub fn with_new_tests(
        &self,
        run_tests: Option<&[String]>,
        exclude_tests: Option<&[String]>,
    ) -> Self {
        let mut config = self.clone();
        let mut updated_selector = self.selector.clone();
        if let Some(exclude_tests) = exclude_tests {
            let mut files_to_exclude = vec![];
            if let Some(excluded_files) = &updated_selector.exclude_files {
                files_to_exclude.extend(excluded_files);
            }
            files_to_exclude.extend(exclude_tests.iter());
            updated_selector.exclude_files = Some(
                files_to_exclude
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            );
        } else if let Some(run_tests) = run_tests {
            updated_selector.exclude_files = None;
            updated_selector.test_root = Some(TestRoot::Roots {
                roots: run_tests.iter().map(|s| s.to_string()).collect(),
            });
        }

        config.selector = updated_selector;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // with_new_tests tests
    #[test]
    fn test_with_new_tests_can_add_tests_to_exclude_list() {
        let config_yaml = "
            test_kind: js_test

            selector:
              roots:
                - jstests/auth/*.js
              exclude_files:
                - jstests/auth/repl.js
                - jstests/core/add1.js
        
            executor:
              config:
                value
              fixture:
                class: MyFixture
                num_nodes: 3
        ";

        let exclude_test_list = vec!["test0.js".to_string(), "test1.js".to_string()];

        let resmoke_suite = ResmokeSuiteConfig::from_str(config_yaml).unwrap();
        let new_config = resmoke_suite.with_new_tests(None, Some(&exclude_test_list));

        assert!(new_config.selector.exclude_files.is_some());
        if let Some(excluded_files) = new_config.selector.exclude_files {
            for test in exclude_test_list {
                assert!(excluded_files.contains(&test));
            }
        }
    }

    #[test]
    fn test_with_new_tests_can_add_tests_to_test_root() {
        let config_yaml = "
            test_kind: js_test

            selector:
              roots:
                - jstests/auth/*.js
              exclude_files:
                - jstests/auth/repl.js
                - jstests/core/add1.js
        
            executor:
              config:
                value
              fixture:
                class: MyFixture
                num_nodes: 3
        ";

        let new_test_list = vec!["test0.js".to_string(), "test1.js".to_string()];

        let resmoke_suite = ResmokeSuiteConfig::from_str(config_yaml).unwrap();
        let new_config = resmoke_suite.with_new_tests(Some(&new_test_list), None);

        if let Some(TestRoot::Roots { roots: test_roots }) = new_config.selector.test_root {
            for test in new_test_list {
                assert!(test_roots.contains(&test));
            }
        } else {
            panic!(
                "New test root is not expected: {:?}",
                new_config.selector.test_root
            );
        }
    }
}
