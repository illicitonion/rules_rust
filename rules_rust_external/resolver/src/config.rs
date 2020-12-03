use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use semver::VersionReq;
use serde::{Deserialize, Serialize};

use crate::consolidator::{ConsolidatorConfig, ConsolidatorOverride};
use crate::parser::merge_cargo_tomls;
use crate::renderer::RenderConfig;
use crate::resolver::{Resolver, ResolverConfig};

#[derive(Debug, Deserialize, Serialize, Ord, Eq, PartialOrd, PartialEq)]
pub struct Package {
    pub name: String,
    pub semver: VersionReq,
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    // Mapping of environment variables key -> value.
    pub extra_rust_env_vars: BTreeMap<String, String>,
    // Mapping of environment variables key -> value.
    pub extra_build_script_env_vars: BTreeMap<String, String>,
    // Mapping of target triple or spec -> extra bazel target dependencies.
    pub extra_bazel_deps: BTreeMap<String, Vec<String>>,
    // Mapping of target triple or spec -> extra bazel target data dependencies.
    pub extra_bazel_data_deps: BTreeMap<String, Vec<String>>,
    // Mapping of target triple or spec -> extra bazel target build script dependencies.
    pub extra_build_script_bazel_deps: BTreeMap<String, Vec<String>>,
    // Mapping of target triple or spec -> extra bazel target build script data dependencies.
    pub extra_build_script_bazel_data_deps: BTreeMap<String, Vec<String>>,
    // Features to remove from crates (e.g. which are needed when building with Cargo but not with Bazel).
    pub features_to_remove: BTreeSet<String>,
}

// Options which affect the contents of the generated output should be on this struct.
// These fields all end up hashed into the lockfile hash.
//
// Anything which doesn't affect the contents of the generated output should live on `Opt` in `main.rs`.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub packages: Vec<Package>,
    pub cargo_toml_files: BTreeMap<String, PathBuf>,
    pub overrides: HashMap<String, Override>,
    pub repository_template: String,
    pub target_triples: Vec<String>,
    pub cargo: PathBuf,
}

impl Config {
    pub fn preprocess(mut self) -> anyhow::Result<Resolver> {
        self.packages.sort();

        let (toml_contents, label_to_crates) =
            merge_cargo_tomls(self.cargo_toml_files, self.packages)?;

        let overrides = self
            .overrides
            .into_iter()
            .map(|(krate, overryde)| {
                (
                    krate,
                    ConsolidatorOverride {
                        extra_rust_env_vars: overryde.extra_rust_env_vars,
                        extra_build_script_env_vars: overryde.extra_build_script_env_vars,
                        extra_bazel_deps: overryde.extra_bazel_deps,
                        extra_build_script_bazel_deps: overryde.extra_build_script_bazel_deps,
                        extra_bazel_data_deps: overryde.extra_bazel_data_deps,
                        extra_build_script_bazel_data_deps: overryde
                            .extra_build_script_bazel_data_deps,
                        features_to_remove: overryde.features_to_remove,
                    },
                )
            })
            .collect();

        Ok(Resolver::new(
            toml_contents.into(),
            ResolverConfig { cargo: self.cargo },
            ConsolidatorConfig { overrides },
            RenderConfig {
                repository_template: self.repository_template.clone(),
            },
            self.target_triples,
            label_to_crates,
        ))
    }
}

// TODO: maybe remove the "+buildmetadata" suffix to consolidate e.g. "1.2.3+foo" and "1.2.3".
/// Generate the repo rule name from the target like cargo-raze.
/// e.g. `0.18.0-alpha.2+test` -> `0_18_0_alpha_2_test`.
pub fn crate_to_repo_rule_name(name: &str, version: &str) -> String {
    format!(
        "__{name}__{version}",
        name = name.replace("-", "_"),
        version = version
            .replace(".", "_")
            .replace("+", "_")
            .replace("-", "_")
    )
}

pub fn crate_to_label(crate_name: &str, crate_version: &str) -> String {
    format!(
        "@{repo_name}//:{name}",
        repo_name = crate_to_repo_rule_name(crate_name, crate_version),
        name = crate_name.replace("-", "_")
    )
}
