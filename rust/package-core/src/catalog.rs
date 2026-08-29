#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[cfg(windows)]
use crate::{
    parse_signature_envelope, parse_trusted_keys, read_installed_lockfile,
    read_repository_sequence_state, repository_sequence_is_acceptable,
    verify_repository_index_envelope, SignedObject, MAX_MANIFEST_BYTES,
};
use crate::{
    LockEntry, Manifest, PackageId, PackageType, RepositoryIndex, SafeRelativePackagePath,
};
#[cfg(windows)]
use std::path::Path;

const BUNDLED_SUMMARY: &str = "Bundled with Fcitx5 for Windows Next";
const CONFIG_SCHEMA: &str = "generic-fcitx-config-v1";

/// A bundled package detected in the installed product layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundledPackage {
    id: PackageId,
    title: String,
    version: String,
}

impl BundledPackage {
    #[must_use]
    pub fn new(id: PackageId, title: String, version: String) -> Self {
        Self { id, title, version }
    }

    #[must_use]
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// The verified repository state or the stable error that made it unavailable.
#[derive(Clone, Copy, Debug)]
pub enum PackageCatalogRepositorySource<'a> {
    Verified(&'a RepositoryIndex),
    Unavailable(PackageCatalogRepositoryError),
}

/// Stable repository failures exposed by package list and detail reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageCatalogRepositoryError {
    RepositoryUnavailable,
    InvalidSignature,
    MissingKey,
    InvalidKeyring,
    InvalidRepository,
    UntrustedKey,
    SequenceStateCorrupt,
    SequenceStateMissing,
    RollbackRejected,
    IoError,
}

impl PackageCatalogRepositoryError {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepositoryUnavailable => "repository_unavailable",
            Self::InvalidSignature => "invalid_signature",
            Self::MissingKey => "missing_key",
            Self::InvalidKeyring => "invalid_keyring",
            Self::InvalidRepository => "invalid_repository",
            Self::UntrustedKey => "untrusted_key",
            Self::SequenceStateCorrupt => "sequence_state_corrupt",
            Self::SequenceStateMissing => "sequence_state_missing",
            Self::RollbackRejected => "rollback_rejected",
            Self::IoError => "io_error",
        }
    }
}

/// Inputs for the package catalog projection.
#[derive(Clone, Copy, Debug)]
pub struct PackageCatalogSources<'a> {
    pub repository: PackageCatalogRepositorySource<'a>,
    pub installed: &'a [LockEntry],
    pub installed_manifests: &'a [Manifest],
    pub bundled: &'a [BundledPackage],
    pub architecture: &'a str,
}

/// Repository metadata retained by a catalog built from verified metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCatalogRepository {
    format_version: u64,
    repository_id: String,
    channel: String,
    mirror_id: String,
    sequence: u64,
    generated_at: String,
    expires_at: String,
    key_id: String,
    targets_sha256: String,
}

impl PackageCatalogRepository {
    fn from_index(index: &RepositoryIndex) -> Self {
        Self {
            format_version: index.format_version(),
            repository_id: index.repository_id().to_owned(),
            channel: index.channel().to_owned(),
            mirror_id: index.mirror_id().to_owned(),
            sequence: index.sequence(),
            generated_at: index.generated_at().to_owned(),
            expires_at: index.expires_at().to_owned(),
            key_id: index.key_id().to_owned(),
            targets_sha256: index.targets_sha256().as_str().to_owned(),
        }
    }

    #[must_use]
    pub fn format_version(&self) -> u64 {
        self.format_version
    }

    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }

    #[must_use]
    pub fn mirror_id(&self) -> &str {
        &self.mirror_id
    }

    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    #[must_use]
    pub fn expires_at(&self) -> &str {
        &self.expires_at
    }

    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    #[must_use]
    pub fn targets_sha256(&self) -> &str {
        &self.targets_sha256
    }
}

/// A dependency shown by a package catalog entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCatalogDependency {
    id: String,
    version: String,
}

impl PackageCatalogDependency {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// A configuration surface exposed by a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageConfigSurface {
    kind: String,
    owner: String,
}

impl PackageConfigSurface {
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    #[must_use]
    pub fn schema(&self) -> &'static str {
        CONFIG_SCHEMA
    }
}

/// One deterministic package list and detail record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCatalogEntry {
    id: String,
    title: String,
    summary: String,
    detail_title: String,
    detail_summary: String,
    package_type: PackageCatalogPackageType,
    detail_package_type: PackageType,
    available_version: Option<String>,
    installed_version: Option<String>,
    state: Option<String>,
    bundled: bool,
    update_available: bool,
    manifest_sha256: Option<String>,
    source_commit: Option<String>,
    dependencies: Vec<PackageCatalogDependency>,
    permissions: Vec<String>,
    config_surfaces: Vec<PackageConfigSurface>,
}

impl PackageCatalogEntry {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    #[must_use]
    pub fn detail_title(&self) -> &str {
        &self.detail_title
    }

    #[must_use]
    pub fn detail_summary(&self) -> &str {
        &self.detail_summary
    }

    #[must_use]
    pub fn package_type(&self) -> &PackageCatalogPackageType {
        &self.package_type
    }

    #[must_use]
    pub fn detail_package_type(&self) -> &PackageType {
        &self.detail_package_type
    }

    #[must_use]
    pub fn available_version(&self) -> Option<&str> {
        self.available_version.as_deref()
    }

    #[must_use]
    pub fn installed_version(&self) -> Option<&str> {
        self.installed_version.as_deref()
    }

    #[must_use]
    pub fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    #[must_use]
    pub fn bundled(&self) -> bool {
        self.bundled
    }

    #[must_use]
    pub fn update_available(&self) -> bool {
        self.update_available
    }

    #[must_use]
    pub fn manifest_sha256(&self) -> Option<&str> {
        self.manifest_sha256.as_deref()
    }

    #[must_use]
    pub fn source_commit(&self) -> Option<&str> {
        self.source_commit.as_deref()
    }

    #[must_use]
    pub fn dependencies(&self) -> &[PackageCatalogDependency] {
        &self.dependencies
    }

    #[must_use]
    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    #[must_use]
    pub fn config_surfaces(&self) -> &[PackageConfigSurface] {
        &self.config_surfaces
    }
}

/// Read-only package list and detail model for Control and Settings consumers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCatalog {
    repository: Option<PackageCatalogRepository>,
    repository_error: Option<PackageCatalogRepositoryError>,
    packages: Vec<PackageCatalogEntry>,
}

impl PackageCatalog {
    #[must_use]
    pub fn from_sources(sources: PackageCatalogSources<'_>) -> Self {
        let (repository, repository_error, repository_entries) = match sources.repository {
            PackageCatalogRepositorySource::Verified(index) => (
                Some(PackageCatalogRepository::from_index(index)),
                None,
                index
                    .packages()
                    .iter()
                    .filter(|entry| {
                        entry.architecture() == "any"
                            || entry.architecture() == sources.architecture
                    })
                    .fold(BTreeMap::new(), |mut entries, entry| {
                        entries.entry(entry.id()).or_insert(entry);
                        entries
                    }),
            ),
            PackageCatalogRepositorySource::Unavailable(error) => {
                (None, Some(error), BTreeMap::new())
            }
        };
        let installed = sources
            .installed
            .iter()
            .map(|entry| (entry.id().as_str(), entry))
            .collect::<BTreeMap<_, _>>();
        let manifests = sources
            .installed_manifests
            .iter()
            .map(|manifest| ((manifest.id().as_str(), manifest.version()), manifest))
            .collect::<BTreeMap<_, _>>();
        let bundled = sources
            .bundled
            .iter()
            .map(|package| (package.id().as_str(), package))
            .collect::<BTreeMap<_, _>>();
        let ids = repository_entries
            .keys()
            .copied()
            .chain(installed.keys().copied())
            .chain(bundled.keys().copied())
            .collect::<BTreeSet<_>>();
        let packages = ids
            .into_iter()
            .map(|id| {
                let repository_entry = repository_entries.get(id).copied();
                let installed_entry = installed.get(id).copied();
                let bundled_package = bundled.get(id).copied();
                let manifest = installed_entry.and_then(|entry| {
                    manifests
                        .get(&(entry.id().as_str(), entry.version()))
                        .copied()
                });
                PackageCatalogEntry::from_sources(
                    id,
                    repository_entry,
                    installed_entry,
                    bundled_package,
                    manifest,
                )
            })
            .collect();
        Self {
            repository,
            repository_error,
            packages,
        }
    }

    #[must_use]
    pub fn repository_available(&self) -> bool {
        self.repository.is_some()
    }

    #[must_use]
    pub fn repository_error(&self) -> Option<PackageCatalogRepositoryError> {
        self.repository_error
    }

    #[must_use]
    pub fn repository(&self) -> Option<&PackageCatalogRepository> {
        self.repository.as_ref()
    }

    #[must_use]
    pub fn packages(&self) -> &[PackageCatalogEntry] {
        &self.packages
    }

    #[must_use]
    pub fn package(&self, id: &str) -> Option<&PackageCatalogEntry> {
        self.packages
            .binary_search_by(|entry| entry.id.as_str().cmp(id))
            .ok()
            .map(|index| &self.packages[index])
    }
}

/// A product-owned bundled component and its safe relative presence probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundledPackageProbe {
    id: PackageId,
    title: String,
    relative_path: SafeRelativePackagePath,
}

impl BundledPackageProbe {
    #[must_use]
    pub fn new(id: PackageId, title: String, relative_path: SafeRelativePackagePath) -> Self {
        Self {
            id,
            title,
            relative_path,
        }
    }
}

/// Selects the production repository cache or an already-verified test/integration boundary.
#[derive(Clone, Copy, Debug)]
pub enum PackageCatalogRepositoryRead<'a> {
    Cached,
    Verified(&'a RepositoryIndex),
}

/// Product identity and platform inputs required by the filesystem reader.
#[derive(Clone, Copy, Debug)]
pub struct PackageCatalogReadOptions<'a> {
    pub expected_channel: &'a str,
    pub architecture: &'a str,
    pub release_version: &'a str,
    pub bundled: &'a [BundledPackageProbe],
    pub repository: PackageCatalogRepositoryRead<'a>,
}

/// Fatal package catalog/detail lookup failures. Repository failures remain catalog state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageCatalogReadError {
    InvalidLockfile,
    InvalidPackage,
    PackageNotFound,
}

impl PackageCatalogReadError {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidLockfile => "invalid_lockfile",
            Self::InvalidPackage => "invalid_package",
            Self::PackageNotFound => "package_not_found",
        }
    }
}

impl std::fmt::Display for PackageCatalogReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for PackageCatalogReadError {}

#[cfg(windows)]
pub fn read_package_catalog(
    install_root: impl AsRef<Path>,
    data_root: impl AsRef<Path>,
    options: PackageCatalogReadOptions<'_>,
) -> Result<PackageCatalog, PackageCatalogReadError> {
    let install_root = install_root.as_ref();
    let data_root = data_root.as_ref();
    let package_root = data_root.join("packages");
    let installed = read_installed_lockfile(&package_root)
        .map_err(|_| PackageCatalogReadError::InvalidLockfile)?;
    let installed_manifests = read_catalog_manifests(&package_root, &installed);
    let bundled = options
        .bundled
        .iter()
        .filter(|probe| install_root.join(probe.relative_path.as_str()).is_file())
        .map(|probe| {
            BundledPackage::new(
                probe.id.clone(),
                probe.title.clone(),
                options.release_version.to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let repository = match options.repository {
        PackageCatalogRepositoryRead::Verified(repository) => {
            PackageCatalogRepositorySource::Verified(repository)
        }
        PackageCatalogRepositoryRead::Cached => {
            match read_cached_repository(install_root, data_root, options.expected_channel) {
                Ok(repository) => {
                    return Ok(PackageCatalog::from_sources(PackageCatalogSources {
                        repository: PackageCatalogRepositorySource::Verified(&repository),
                        installed: &installed,
                        installed_manifests: &installed_manifests,
                        bundled: &bundled,
                        architecture: options.architecture,
                    }));
                }
                Err(error) => PackageCatalogRepositorySource::Unavailable(error),
            }
        }
    };
    Ok(PackageCatalog::from_sources(PackageCatalogSources {
        repository,
        installed: &installed,
        installed_manifests: &installed_manifests,
        bundled: &bundled,
        architecture: options.architecture,
    }))
}

#[cfg(windows)]
pub fn read_package_detail(
    install_root: impl AsRef<Path>,
    data_root: impl AsRef<Path>,
    package_id: &str,
    options: PackageCatalogReadOptions<'_>,
) -> Result<PackageCatalogEntry, PackageCatalogReadError> {
    PackageId::parse(package_id).map_err(|_| PackageCatalogReadError::InvalidPackage)?;
    read_package_catalog(install_root, data_root, options)?
        .package(package_id)
        .cloned()
        .ok_or(PackageCatalogReadError::PackageNotFound)
}

#[cfg(windows)]
fn read_catalog_manifests(package_root: &Path, installed: &[LockEntry]) -> Vec<Manifest> {
    installed
        .iter()
        .filter_map(|entry| {
            let path = package_root
                .join("manifests")
                .join(entry.id().as_str())
                .join(format!("{}.json", entry.version()));
            let bytes = crate::read_repair_file_bounded(&path, MAX_MANIFEST_BYTES as u64).ok()?;
            if crate::sha256_digest(&bytes) != *entry.manifest_sha256() {
                return None;
            }
            let manifest = crate::parse_manifest(std::str::from_utf8(&bytes).ok()?).ok()?;
            (manifest.id() == entry.id() && manifest.version() == entry.version())
                .then_some(manifest)
        })
        .collect()
}

#[cfg(windows)]
fn read_cached_repository(
    install_root: &Path,
    data_root: &Path,
    expected_channel: &str,
) -> Result<RepositoryIndex, PackageCatalogRepositoryError> {
    let index = crate::read_repair_file_bounded(
        &data_root.join("repository/index.json"),
        MAX_MANIFEST_BYTES as u64,
    )
    .map_err(|_| PackageCatalogRepositoryError::RepositoryUnavailable)?;
    let signature = crate::read_repair_file_bounded(
        &data_root.join("repository/index.sig.json"),
        MAX_MANIFEST_BYTES as u64,
    )
    .map_err(|_| PackageCatalogRepositoryError::InvalidSignature)?;
    let keyring_path = install_root.join("security/trusted-keys.json");
    if !keyring_path.is_file() {
        return Err(PackageCatalogRepositoryError::MissingKey);
    }
    let keyring = crate::read_repair_file_bounded(&keyring_path, MAX_MANIFEST_BYTES as u64)
        .map_err(|_| PackageCatalogRepositoryError::IoError)?;
    let trusted_keys = parse_trusted_keys(
        std::str::from_utf8(&keyring).map_err(|_| PackageCatalogRepositoryError::InvalidKeyring)?,
    )
    .map_err(|_| PackageCatalogRepositoryError::InvalidKeyring)?;
    let envelope = parse_signature_envelope(
        std::str::from_utf8(&signature)
            .map_err(|_| PackageCatalogRepositoryError::InvalidSignature)?,
        SignedObject::RepositoryIndex,
    )
    .map_err(|_| PackageCatalogRepositoryError::InvalidSignature)?;
    let repository =
        verify_repository_index_envelope(&index, &envelope, &trusted_keys, expected_channel)
            .map_err(|error| repository_error_from_code(error.code()))?;
    let sequence = read_repository_sequence_state(data_root, repository.channel());
    if !sequence.present {
        return Err(PackageCatalogRepositoryError::SequenceStateMissing);
    }
    if !sequence.valid {
        return Err(PackageCatalogRepositoryError::SequenceStateCorrupt);
    }
    let maximum = repository
        .packages()
        .iter()
        .map(|entry| entry.release_sequence())
        .max()
        .unwrap_or(0);
    if !repository_sequence_is_acceptable(repository.sequence(), sequence.maximum, false)
        || !repository_sequence_is_acceptable(maximum, sequence.maximum, false)
    {
        return Err(PackageCatalogRepositoryError::RollbackRejected);
    }
    Ok(repository)
}

#[cfg(windows)]
fn repository_error_from_code(code: &str) -> PackageCatalogRepositoryError {
    match code {
        "invalid_signature" => PackageCatalogRepositoryError::InvalidSignature,
        "untrusted_key" => PackageCatalogRepositoryError::UntrustedKey,
        "invalid_repository" => PackageCatalogRepositoryError::InvalidRepository,
        _ => PackageCatalogRepositoryError::InvalidRepository,
    }
}

impl PackageCatalogEntry {
    fn from_sources(
        id: &str,
        repository: Option<&crate::RepositoryEntry>,
        installed: Option<&LockEntry>,
        bundled: Option<&BundledPackage>,
        manifest: Option<&Manifest>,
    ) -> Self {
        let detail_package_type = manifest
            .map(|manifest| manifest.package_type().clone())
            .or_else(|| repository.map(|entry| entry.package_type().clone()))
            .unwrap_or(PackageType::Addon);
        let package_type = repository
            .map(|entry| PackageCatalogPackageType::Known(entry.package_type().clone()))
            .or_else(|| {
                (installed.is_none() && bundled.is_some())
                    .then_some(PackageCatalogPackageType::Known(PackageType::Addon))
            })
            .unwrap_or(PackageCatalogPackageType::Unknown);
        let dependencies = manifest
            .map(|manifest| {
                manifest
                    .dependencies()
                    .iter()
                    .map(|dependency| PackageCatalogDependency {
                        id: dependency.id().as_str().to_owned(),
                        version: dependency.version().to_owned(),
                    })
                    .collect()
            })
            .or_else(|| {
                repository.map(|entry| {
                    entry
                        .dependencies()
                        .iter()
                        .map(|dependency| PackageCatalogDependency {
                            id: dependency.id().to_owned(),
                            version: dependency.version().to_owned(),
                        })
                        .collect()
                })
            })
            .unwrap_or_default();
        let permissions = manifest
            .map(|manifest| manifest.permissions().to_vec())
            .unwrap_or_default();
        let repository_title = repository.map(|entry| entry.title());
        let repository_summary = repository.map(|entry| entry.summary());
        Self {
            id: id.to_owned(),
            title: repository_title
                .map(str::to_owned)
                .or_else(|| {
                    installed
                        .is_none()
                        .then(|| bundled.map(|package| package.title().to_owned()))
                        .flatten()
                })
                .unwrap_or_else(|| id.to_owned()),
            summary: repository_summary
                .map(str::to_owned)
                .or_else(|| {
                    (installed.is_none() && bundled.is_some()).then(|| BUNDLED_SUMMARY.to_owned())
                })
                .unwrap_or_default(),
            detail_title: repository_title.unwrap_or(id).to_owned(),
            detail_summary: repository_summary
                .map(str::to_owned)
                .or_else(|| bundled.map(|_| BUNDLED_SUMMARY.to_owned()))
                .unwrap_or_default(),
            package_type,
            detail_package_type: detail_package_type.clone(),
            available_version: repository.map(|entry| entry.version().to_owned()),
            installed_version: installed
                .map(|entry| entry.version().to_owned())
                .or_else(|| bundled.map(|package| package.version().to_owned())),
            state: installed
                .map(|entry| entry.state().as_str().to_owned())
                .or_else(|| bundled.map(|_| "bundled".to_owned())),
            bundled: bundled.is_some(),
            update_available: installed.is_some_and(|entry| {
                repository.is_some_and(|available| {
                    !entry.version().is_empty()
                        && !available.version().is_empty()
                        && entry.version() != available.version()
                })
            }),
            manifest_sha256: installed.map(|entry| entry.manifest_sha256().as_str().to_owned()),
            source_commit: manifest.map(|manifest| manifest.source_commit().to_owned()),
            dependencies,
            permissions,
            config_surfaces: config_surfaces(id, &detail_package_type, manifest),
        }
    }
}

/// The package type used in package-list output, including installed-only records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageCatalogPackageType {
    Known(PackageType),
    Unknown,
}

impl PackageCatalogPackageType {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(package_type) => package_type.as_str(),
            Self::Unknown => "unknown",
        }
    }
}

fn config_surfaces(
    id: &str,
    package_type: &PackageType,
    manifest: Option<&Manifest>,
) -> Vec<PackageConfigSurface> {
    let mut kinds = BTreeSet::new();
    match package_type {
        PackageType::Theme => {
            kinds.insert("theme");
        }
        PackageType::InputMethodData => {
            kinds.insert("input-method-data");
        }
        PackageType::Addon => {
            kinds.insert("fcitx-addon");
        }
        PackageType::Core | PackageType::Translation => {}
    }
    if let Some(manifest) = manifest {
        if manifest
            .permissions()
            .iter()
            .any(|permission| permission == "input-data")
        {
            kinds.insert("input-method-data");
        }
        for file in manifest.files() {
            let path = file.path().as_str();
            if path.starts_with("share/fcitx5/addon/") && path.ends_with(".conf") {
                kinds.insert("fcitx-addon-config");
            }
            if path.starts_with("lib/fcitx5/") && path.ends_with(".dll") {
                kinds.insert("fcitx-addon");
            }
            if path.starts_with("share/rime-data/") {
                kinds.insert("rime-data");
            }
            if path.starts_with("themes/") || path.starts_with("share/themes/") {
                kinds.insert("theme");
            }
        }
    }
    kinds
        .into_iter()
        .map(|kind| PackageConfigSurface {
            kind: kind.to_owned(),
            owner: id.to_owned(),
        })
        .collect()
}
