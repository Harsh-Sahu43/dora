use crate::{
    package_metadata::{DependencyRequirement, PackageDefinition, PackageIdentity},
    registry::{RegistryIndex, RegistryPackageRecord},
};
use eyre::bail;
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageKey {
    pub name: String,
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNode {
    pub package: PackageDefinition,
    pub dependencies: BTreeMap<String, PackageKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionGraph {
    pub root: PackageKey,
    pub nodes: BTreeMap<PackageKey, ResolvedNode>,
}

pub fn resolve_dependencies(
    root: &PackageDefinition,
    index: &RegistryIndex,
) -> eyre::Result<ResolutionGraph> {
    let root_key = PackageKey::from_identity(&root.identity);
    let mut state = ResolverState {
        index,
        nodes: BTreeMap::new(),
        selected_versions: BTreeMap::from([(
            root.identity.name.clone(),
            root.identity.version.clone(),
        )]),
        visiting: BTreeSet::new(),
    };

    state.resolve_package(root.clone())?;

    Ok(ResolutionGraph {
        root: root_key,
        nodes: state.nodes,
    })
}

struct ResolverState<'a> {
    index: &'a RegistryIndex,
    nodes: BTreeMap<PackageKey, ResolvedNode>,
    selected_versions: BTreeMap<String, Version>,
    visiting: BTreeSet<PackageKey>,
}

impl ResolverState<'_> {
    fn resolve_package(&mut self, package: PackageDefinition) -> eyre::Result<PackageKey> {
        let key = PackageKey::from_identity(&package.identity);
        if self.nodes.contains_key(&key) {
            return Ok(key);
        }
        if !self.visiting.insert(key.clone()) {
            return Ok(key);
        }

        let mut dependencies = BTreeMap::new();
        for (dep_name, requirement) in &package.dependencies {
            let selected = match requirement {
                DependencyRequirement::Version { requirement } => {
                    self.resolve_version_dependency(dep_name, requirement)?
                }
                DependencyRequirement::Path { .. } => {
                    bail!(
                        "dependency `{dep_name}` uses `path`, which is not supported by the resolver yet"
                    );
                }
                DependencyRequirement::Git { .. } => {
                    bail!(
                        "dependency `{dep_name}` uses `git`, which is not supported by the resolver yet"
                    );
                }
            };
            dependencies.insert(dep_name.clone(), selected);
        }

        self.visiting.remove(&key);
        self.nodes.insert(
            key.clone(),
            ResolvedNode {
                package,
                dependencies,
            },
        );
        Ok(key)
    }

    fn resolve_version_dependency(
        &mut self,
        dep_name: &str,
        requirement: &VersionReq,
    ) -> eyre::Result<PackageKey> {
        let Some(selected) = select_registry_package(self.index, dep_name, requirement) else {
            bail!("no registry package found for dependency `{dep_name}` matching `{requirement}`");
        };

        let selected_key = PackageKey::from_identity(&selected.package.identity);
        if let Some(existing) = self.selected_versions.get(dep_name) {
            if existing != &selected_key.version {
                bail!(
                    "dependency version conflict for `{dep_name}`: selected `{existing}`, but `{}` was also required",
                    selected_key.version
                );
            }
        } else {
            self.selected_versions
                .insert(dep_name.to_owned(), selected_key.version.clone());
        }

        self.resolve_package(selected.package.clone())?;
        Ok(selected_key)
    }
}

fn select_registry_package<'a>(
    index: &'a RegistryIndex,
    name: &str,
    requirement: &VersionReq,
) -> Option<&'a RegistryPackageRecord> {
    let mut matching = index
        .packages
        .iter()
        .filter(|pkg| pkg.package.identity.name == name)
        .filter(|pkg| requirement.matches(&pkg.package.identity.version))
        .collect::<Vec<_>>();

    matching.sort_by(|a, b| b.package.identity.version.cmp(&a.package.identity.version));
    matching.into_iter().next()
}

impl PackageKey {
    fn from_identity(identity: &PackageIdentity) -> Self {
        Self {
            name: identity.name.clone(),
            version: identity.version.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        package_metadata::{DependencyRequirement, PackageDefinition, PackageIdentity},
        registry::{RegistryIndex, RegistryPackageRecord},
    };

    fn package(name: &str, version: &str, deps: &[(&str, &str)]) -> PackageDefinition {
        PackageDefinition {
            identity: PackageIdentity {
                name: name.to_owned(),
                version: Version::parse(version).unwrap(),
            },
            entrypoint: format!("{name}.main"),
            description: None,
            dependencies: deps
                .iter()
                .map(|(dep_name, req)| {
                    (
                        (*dep_name).to_owned(),
                        DependencyRequirement::Version {
                            requirement: VersionReq::parse(req).unwrap(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn registry(packages: Vec<PackageDefinition>) -> RegistryIndex {
        RegistryIndex {
            version: 1,
            packages: packages
                .into_iter()
                .map(|package| RegistryPackageRecord { package })
                .collect(),
        }
    }

    #[test]
    fn resolves_single_version_dependency() {
        let root = package("camera_node", "0.1.0", &[("yolo", "^1.2.0")]);
        let index = registry(vec![package("yolo", "1.2.3", &[])]);

        let resolved = resolve_dependencies(&root, &index).unwrap();
        assert_eq!(resolved.nodes.len(), 2);
        assert!(resolved.nodes.contains_key(&PackageKey {
            name: "yolo".into(),
            version: Version::parse("1.2.3").unwrap()
        }));
    }

    #[test]
    fn resolves_transitive_dependencies() {
        let root = package("camera_node", "0.1.0", &[("yolo", "^1.2.0")]);
        let index = registry(vec![
            package("yolo", "1.2.3", &[("tensor", "^2.0.0")]),
            package("tensor", "2.1.0", &[]),
        ]);

        let resolved = resolve_dependencies(&root, &index).unwrap();
        assert_eq!(resolved.nodes.len(), 3);
        let yolo = resolved
            .nodes
            .get(&PackageKey {
                name: "yolo".into(),
                version: Version::parse("1.2.3").unwrap(),
            })
            .unwrap();
        assert_eq!(
            yolo.dependencies["tensor"].version,
            Version::parse("2.1.0").unwrap()
        );
    }

    #[test]
    fn selects_latest_matching_version() {
        let root = package("camera_node", "0.1.0", &[("yolo", "^1.2.0")]);
        let index = registry(vec![
            package("yolo", "1.2.1", &[]),
            package("yolo", "1.4.0", &[]),
            package("yolo", "2.0.0", &[]),
        ]);

        let resolved = resolve_dependencies(&root, &index).unwrap();
        let root_node = resolved.nodes.get(&resolved.root).unwrap();
        assert_eq!(
            root_node.dependencies["yolo"].version,
            Version::parse("1.4.0").unwrap()
        );
    }

    #[test]
    fn fails_when_dependency_is_missing() {
        let root = package("camera_node", "0.1.0", &[("yolo", "^1.2.0")]);
        let index = registry(vec![]);

        let err = resolve_dependencies(&root, &index).unwrap_err().to_string();
        assert!(err.contains("no registry package found"));
    }

    #[test]
    fn fails_when_no_version_matches() {
        let root = package("camera_node", "0.1.0", &[("yolo", "^1.2.0")]);
        let index = registry(vec![package("yolo", "2.0.0", &[])]);

        let err = resolve_dependencies(&root, &index).unwrap_err().to_string();
        assert!(err.contains("no registry package found"));
    }

    #[test]
    fn fails_when_conflicting_versions_are_required() {
        let root = package(
            "camera_node",
            "0.1.0",
            &[("yolo", "^1.2.0"), ("math", "^0.3.0")],
        );
        let index = registry(vec![
            package("yolo", "1.2.3", &[("tensor", "^2.0.0")]),
            package("math", "0.3.1", &[("tensor", "^3.0.0")]),
            package("tensor", "2.1.0", &[]),
            package("tensor", "3.1.0", &[]),
        ]);

        let err = resolve_dependencies(&root, &index).unwrap_err().to_string();
        assert!(err.contains("dependency version conflict"));
    }

    #[test]
    fn fails_on_unsupported_path_dependency() {
        let mut root = package("camera_node", "0.1.0", &[]);
        root.dependencies.insert(
            "local_math".to_owned(),
            DependencyRequirement::Path {
                path: "../math".to_owned(),
            },
        );

        let err = resolve_dependencies(&root, &registry(vec![]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("not supported by the resolver yet"));
    }

    #[test]
    fn fails_on_unsupported_git_dependency() {
        let mut root = package("camera_node", "0.1.0", &[]);
        root.dependencies.insert(
            "vision".to_owned(),
            DependencyRequirement::Git {
                repo: "https://example.com/vision.git".to_owned(),
                rev: Some("main".to_owned()),
            },
        );

        let err = resolve_dependencies(&root, &registry(vec![]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("not supported by the resolver yet"));
    }
}
