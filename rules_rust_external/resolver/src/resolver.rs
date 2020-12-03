use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use cargo_metadata::{DependencyKind, MetadataCommand};
use cargo_raze::context::CrateContext;
use cargo_raze::metadata::{CargoMetadataFetcher, CargoWorkspaceFiles};
use cargo_raze::planning::{BuildPlanner, BuildPlannerImpl};
use cargo_raze::settings::{GenMode, RazeSettings};
use log::trace;
use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};

use crate::consolidator::{Consolidator, ConsolidatorConfig, ConsolidatorOverride};
use crate::renderer::RenderConfig;
use crate::NamedTempFile;
use anyhow::Context;

pub struct ResolverConfig {
    pub cargo: PathBuf,
}

pub struct Resolver {
    pub toml: toml::Value,
    pub resolver_config: ResolverConfig,
    pub consolidator_config: ConsolidatorConfig,
    pub render_config: RenderConfig,
    pub target_triples: Vec<String>,
    pub label_to_crates: BTreeMap<String, BTreeSet<String>>,
    digest: Option<String>,
}

// TODO: Interesting edge cases
// - you can pass deps using: version number path on fs, git repo.
// - you can rename crates you depend on.
pub struct ResolvedArtifactsWithMetadata {
    pub resolved_packages: Vec<CrateContext>,
    pub member_packages_version_mapping: HashMap<String, Version>,
}

impl Resolver {
    pub fn new(
        toml: toml::Value,
        resolver_config: ResolverConfig,
        consolidator_config: ConsolidatorConfig,
        render_config: RenderConfig,
        target_triples: Vec<String>,
        label_to_crates: BTreeMap<String, BTreeSet<String>>,
    ) -> Resolver {
        Resolver {
            toml,
            resolver_config,
            consolidator_config,
            render_config,
            target_triples,
            label_to_crates,
            digest: None,
        }
    }

    pub fn digest(&mut self) -> anyhow::Result<String> {
        // TODO: Ignore * .cargo config files outside of the workspace

        if self.digest.is_none() {
            // TODO: Combine values better
            let mut hasher = Sha256::new();
            // Mix in the version of this crate, which encompasses all logic and templates.
            // This is probably a wild over-estimate of what should go in the cache key.
            // NOTE: In debug mode, this mixes the digest of the executable, rather than the version number.
            hasher.update(version_for_hashing()?);
            hasher.update(b"\0");

            // If new fields are added, you should decide whether they need hashing.
            // Hint: They probably do. If not, please add a comment justifying why not.
            let Self {
                toml,
                render_config:
                    RenderConfig {
                        repository_template,
                    },
                consolidator_config: ConsolidatorConfig { overrides },
                resolver_config: ResolverConfig { cargo },

                // This is what we're computing.
                digest: _ignored,
                target_triples,
                label_to_crates,
            } = &self;

            hasher.update(repository_template.as_bytes());
            hasher.update(b"\0");

            hasher.update(get_cargo_version(&cargo)?);
            hasher.update(b"\0");
            for target_triple in target_triples {
                hasher.update(target_triple);
                hasher.update(b"\0");
            }
            hasher.update(b"\0");
            for (label, crates) in label_to_crates.iter() {
                hasher.update(label.as_bytes());
                hasher.update(b"\0");
                for krate in crates.iter() {
                    hasher.update(krate.as_bytes());
                    hasher.update(b"\0");
                }
            }

            // TODO: improve the caching by generating a lockfile over the resolve rather than over
            // the render. If the digest contains only input for the cargo dependency resolution
            // then we don't need to re-pin when making changes to things that only affect the
            // generated bazel file.
            for (
                crate_name,
                ConsolidatorOverride {
                    extra_rust_env_vars,
                    extra_build_script_env_vars,
                    extra_bazel_deps,
                    extra_bazel_data_deps,
                    extra_build_script_bazel_deps,
                    extra_build_script_bazel_data_deps,
                    features_to_remove,
                },
            ) in overrides
            {
                hasher.update(crate_name);
                hasher.update(b"\0");
                for (env_key, env_val) in extra_rust_env_vars {
                    hasher.update(env_key);
                    hasher.update(b"\0");
                    hasher.update(env_val);
                    hasher.update(b"\0");
                }
                for (env_key, env_val) in extra_build_script_env_vars {
                    hasher.update(env_key);
                    hasher.update(b"\0");
                    hasher.update(env_val);
                    hasher.update(b"\0");
                }
                for dep_map in vec![
                    extra_bazel_deps,
                    extra_bazel_data_deps,
                    extra_build_script_bazel_deps,
                    extra_build_script_bazel_data_deps,
                ] {
                    for (target, deps) in dep_map {
                        hasher.update(target);
                        hasher.update(b"\0");
                        for dep in deps {
                            hasher.update(dep);
                            hasher.update(b"\0");
                        }
                    }
                }
                for feature in features_to_remove {
                    hasher.update(feature);
                    hasher.update(b"\n");
                }
            }

            for (env_name, env_value) in std::env::vars() {
                // The CARGO_HOME variable changes where cargo writes and reads config, and caches.
                // We currently use the user's Cargo home (by not overwriting it) so we should
                // allow users to use a custom path to one.
                if env_name == "CARGO_HOME" {
                    continue;
                }
                // We hope that other env vars don't cause problems...
                if env_name.starts_with("CARGO") && env_name != "CARGO_NET_GIT_FETCH_WITH_CLI" {
                    eprintln!("Warning: You have the {} environment variable set - this may affect your rules_rust_external output", env_name);
                    hasher.update(env_name);
                    hasher.update(b"\0");
                    hasher.update(env_value);
                    hasher.update(b"\0");
                }
            }

            hasher.update(toml.to_string().as_bytes());
            hasher.update(b"\0");

            // TODO: Include all files referenced by the toml.
            self.digest = Some(hex::encode(hasher.finalize()));
        }
        // UNWRAP: Guaranteed by above code.
        Ok(self.digest.clone().unwrap())
    }

    pub fn resolve(mut self) -> anyhow::Result<Consolidator> {
        let toml_str = self.toml.to_string();
        trace!("Resolving for generated Cargo.toml:\n{}", toml_str);
        let merged_cargo_toml = NamedTempFile::with_str_content("Cargo.toml", &toml_str)
            .context("Writing intermediate Cargo.toml")?;

        let mut md_fetcher = CargoMetadataFetcher::new(&self.resolver_config.cargo, false);

        let cargo_ws_files = CargoWorkspaceFiles {
            toml_path: PathBuf::from(merged_cargo_toml.path()),
            lock_path_opt: None,
        };
        let mut planner = BuildPlannerImpl::new(&mut md_fetcher);

        // TODO: These are ?all ignored
        let raze_settings = RazeSettings {
            workspace_path: "".to_string(),
            incompatible_relative_workspace_path: false,
            target: None,
            targets: Some(self.target_triples.clone()),
            crates: HashMap::default(),
            gen_workspace_prefix: "".to_string(),
            genmode: GenMode::Remote,
            output_buildfile_suffix: "".to_string(),
            default_gen_buildrs: true,
            registry: "".to_string(),
            binary_deps: HashMap::default(),
            index_url: String::from("https://github.com/rust-lang/crates.io-index"),
        };
        let planned_build = planner
            .plan_build(&raze_settings, &PathBuf::new(), cargo_ws_files, None)
            .context("Failed planning build")?;

        let mut resolved_packages = planned_build.crate_contexts;
        resolved_packages
            .sort_by(|l, r| (&l.pkg_name, &l.pkg_version).cmp(&(&r.pkg_name, &r.pkg_version)));

        let member_packages_version_mapping =
            self.get_member_packages_version_mapping(merged_cargo_toml.path(), &resolved_packages);

        // TODO: generate a cargo toml from metadata in the bazel rule, when no cargo toml is present.

        let digest = self.digest().context("Digesting Resolver inputs")?;
        Ok(Consolidator::new(
            self.consolidator_config,
            self.render_config,
            digest,
            self.target_triples,
            resolved_packages,
            member_packages_version_mapping?,
            self.label_to_crates,
        ))
    }

    fn get_member_packages_version_mapping(
        &self,
        merged_cargo_toml: &Path,
        resolved_artifacts: &[CrateContext],
    ) -> anyhow::Result<BTreeMap<String, Version>> {
        let merged_cargo_metadata = MetadataCommand::new()
            .cargo_path(&self.resolver_config.cargo)
            .manifest_path(merged_cargo_toml)
            .no_deps()
            .exec()
            .context("Failed to run cargo metadata")?;

        // Build the intersection of version requirements for all the member (i.e. toplevel) packages
        // of our workspace.
        let mut member_package_version_reqs: HashMap<String, Vec<VersionReq>> = HashMap::new();
        for package in &merged_cargo_metadata.packages {
            for dep in &package.dependencies {
                // TODO: Return a map of dep-kind to crate-name to version,
                // so that we can create a build_crate and dev_crate function or similar.
                // Right now we use this result both for the crate() function definition,
                // and the crates_from function definition, but these should be separate.
                if dep.kind == DependencyKind::Normal {
                    let mut cur_version_req = {
                        let empty_vec = vec![];
                        member_package_version_reqs
                            .remove(&dep.name)
                            .unwrap_or(empty_vec)
                    };
                    cur_version_req.push(dep.req.clone());
                    member_package_version_reqs.insert(dep.name.clone(), cur_version_req);
                }
            }
        }

        let mut member_package_version_mapping = BTreeMap::new();
        for package in resolved_artifacts {
            // If the package name matches one of the member packages' direct dependencies, consider it
            // for the final version: insert it into the map if we didn't have one yet, take the highest
            // version so far if there was already one.
            if let Some(version_req) = member_package_version_reqs.get(&package.pkg_name) {
                if version_req
                    .iter()
                    .all(|req| req.matches(&package.pkg_version))
                {
                    let current_pkg_version = member_package_version_mapping
                        .get(&package.pkg_name)
                        .unwrap_or(&Version::new(0, 0, 0))
                        .clone();
                    member_package_version_mapping.insert(
                        package.pkg_name.clone(),
                        current_pkg_version.max(package.pkg_version.clone()),
                    );
                }
            }
        }
        Ok(member_package_version_mapping)
    }
}

fn get_cargo_version(cargo_path: &Path) -> anyhow::Result<Vec<u8>> {
    let output = std::process::Command::new(cargo_path)
        .arg("--version")
        .stderr(Stdio::inherit())
        .output()
        .context("Invoking cargo --version")?;
    if !output.status.success() {
        panic!(
            "TODO: cargo --version failed with exit code {:?}",
            output.status.code()
        );
    }
    Ok(output.stdout)
}

fn version_for_hashing() -> anyhow::Result<Cow<'static, [u8]>> {
    if cfg!(debug_assertions) {
        let current_exe =
            std::env::current_exe().context("Couldn't get current executable path")?;
        Ok(Cow::Owned(
            std::fs::read(current_exe).context("Couldn't read current executable path")?,
        ))
    } else {
        Ok(Cow::Borrowed(env!("CARGO_PKG_VERSION").as_bytes()))
    }
}
