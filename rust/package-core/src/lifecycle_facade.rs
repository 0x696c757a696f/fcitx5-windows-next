#![forbid(unsafe_code)]

//! Public package lifecycle operations for Control and the package command-line tools.
//!
//! This module owns orchestration only. Parsing, cryptographic verification, archive staging,
//! activation, and lockfile mutation remain in their existing authoritative owners.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    activate_installed_version_for_rollback, activate_staged_payload_tree, parse_manifest,
    parse_signature_envelope, parse_trusted_keys, read_installed_lockfile,
    read_repository_sequence_state, sha256_digest, stage_validated_archive_zip, update,
    verify_installed_packages_for_repair, verify_repository_index_envelope_with_policy,
    write_repository_sequence_state, PackageId, PackageType, RepositoryEntry, RepositoryIndex,
    RepositorySequenceState, RepositoryVerificationPolicy, SignedObject, TrustedKey,
    MAX_MANIFEST_BYTES,
};

const MAXIMUM_REPOSITORY_SIGNATURE_BYTES: u64 = 64 * 1024;
const MAXIMUM_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAXIMUM_TRANSACTION_STATE_BYTES: u64 = 1024;

/// A typed failure returned by the public package lifecycle façade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageFacadeError {
    code: &'static str,
    message: String,
}

impl PackageFacadeError {
    /// Creates an error for a transport or transaction boundary implementation.
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the stable machine-readable error code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PackageFacadeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PackageFacadeError {}

/// Rejects endpoints that are not the package service's restricted HTTPS form.
///
/// # Errors
///
/// Returns `invalid_repository_url` unless `value` is an ASCII, credential-free HTTPS URL
/// without a fragment or Windows path separator.
pub fn validate_https_repository_url(value: &str) -> Result<(), PackageFacadeError> {
    if is_credential_free_https_url(value) {
        Ok(())
    } else {
        Err(PackageFacadeError::new(
            "invalid_repository_url",
            "repository endpoint must be credential-free HTTPS without a fragment",
        ))
    }
}

/// Reads and validates a bounded trusted-keyring file.
///
/// # Errors
///
/// Returns an error when the file is unavailable, oversized, not UTF-8, or violates the
/// keyring contract.
pub fn read_trusted_keyring(path: impl AsRef<Path>) -> Result<Vec<TrustedKey>, PackageFacadeError> {
    let bytes = read_bounded_file(path, MAX_MANIFEST_BYTES as u64)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| PackageFacadeError::new("invalid_keyring", "trusted keyring is not UTF-8"))?;
    let keys = parse_trusted_keys(text)
        .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
    if keys.is_empty() {
        return Err(PackageFacadeError::new(
            "invalid_keyring",
            "trusted keyring is empty",
        ));
    }
    Ok(keys)
}

/// Stages an archive through the existing manifest and signature verification owner.
///
/// # Errors
///
/// Returns the existing archive-validation error if the archive is malformed, unsigned, or
/// incompatible with the current keyring.
pub fn stage_verified_archive(
    archive_path: &Path,
    package_root: &Path,
    transaction_id: &str,
    trusted_keys: &[TrustedKey],
) -> Result<PathBuf, PackageFacadeError> {
    stage_validated_archive_zip(archive_path, package_root, transaction_id, trusted_keys)
        .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))
}

/// Atomically activates a previously verified staged payload through the existing owner.
///
/// # Errors
///
/// Returns the existing activation error and leaves recovery to the caller's transaction owner.
pub fn activate_verified_staged_payload(
    staged_root: &Path,
    package_root: &Path,
    trusted_keys: &[TrustedKey],
) -> Result<(), PackageFacadeError> {
    activate_staged_payload_tree(staged_root, package_root, trusted_keys)
        .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))
}

/// The only network seam used by repository refresh and archive download.
///
/// Implementations must return at most `maximum_bytes`. Production callers normally use
/// [`DownloaderRepositoryTransport`]; tests can provide a deterministic fixture transport.
pub trait RepositoryTransport {
    /// Fetches one HTTPS resource after the façade has validated its endpoint.
    fn fetch(&mut self, url: &str, maximum_bytes: u64) -> Result<Vec<u8>, PackageFacadeError>;
}

/// Production adapter that delegates HTTPS transfer to the unelevated downloader executable.
#[derive(Clone, Debug)]
pub struct DownloaderRepositoryTransport {
    downloader_path: PathBuf,
    scratch_root: PathBuf,
    next_file: u64,
}

impl DownloaderRepositoryTransport {
    /// Creates an adapter for the shipping `fcitx5-downloader` executable.
    #[must_use]
    pub fn new(downloader_path: PathBuf, scratch_root: PathBuf) -> Self {
        Self {
            downloader_path,
            scratch_root,
            next_file: 0,
        }
    }
}

impl RepositoryTransport for DownloaderRepositoryTransport {
    fn fetch(&mut self, url: &str, maximum_bytes: u64) -> Result<Vec<u8>, PackageFacadeError> {
        validate_https_repository_url(url)?;
        fs::create_dir_all(&self.scratch_root).map_err(|error| {
            PackageFacadeError::new(
                "io_error",
                format!("download scratch directory failed: {error}"),
            )
        })?;
        let destination = self
            .scratch_root
            .join(format!("repository-{}.download", self.next_file));
        self.next_file = self.next_file.saturating_add(1);
        if destination.exists() {
            return Err(PackageFacadeError::new(
                "io_error",
                "repository download destination already exists",
            ));
        }
        let status = Command::new(&self.downloader_path)
            .arg("--download-signed-metadata")
            .arg(url)
            .arg(&destination)
            .status()
            .map_err(|error| {
                PackageFacadeError::new(
                    "network_error",
                    format!("repository downloader could not start: {error}"),
                )
            })?;
        if !status.success() {
            let _ = fs::remove_file(&destination);
            return Err(PackageFacadeError::new(
                "network_error",
                "repository downloader rejected the endpoint",
            ));
        }
        let result = read_bounded_file(&destination, maximum_bytes)
            .map_err(|error| PackageFacadeError::new(error.code, error.message));
        let _ = fs::remove_file(&destination);
        result
    }
}

/// Immutable product paths used by package lifecycle operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCoreFacade {
    install_root: PathBuf,
    data_root: PathBuf,
    architecture: String,
}

impl PackageCoreFacade {
    /// Creates a façade for one product installation and its durable user data root.
    #[must_use]
    pub fn new(install_root: PathBuf, data_root: PathBuf, architecture: impl Into<String>) -> Self {
        Self {
            install_root,
            data_root,
            architecture: architecture.into(),
        }
    }

    /// Refreshes the verified repository cache without replacing a prior cache on failure.
    pub fn refresh_repository<T: RepositoryTransport>(
        &self,
        request: RepositoryRefreshRequest<'_>,
        transport: &mut T,
    ) -> Result<RepositoryRefreshResult, PackageFacadeError> {
        validate_refresh_request(&request)?;
        let trusted_keys = self.read_production_trusted_keys()?;
        let index_bytes = transport.fetch(request.index_url, MAX_MANIFEST_BYTES as u64)?;
        ensure_bound(&index_bytes, MAX_MANIFEST_BYTES as u64, "repository index")?;
        let signature_bytes =
            transport.fetch(request.signature_url, MAXIMUM_REPOSITORY_SIGNATURE_BYTES)?;
        ensure_bound(
            &signature_bytes,
            MAXIMUM_REPOSITORY_SIGNATURE_BYTES,
            "repository signature",
        )?;
        let envelope_text = std::str::from_utf8(&signature_bytes).map_err(|_| {
            PackageFacadeError::new(
                "invalid_signature",
                "repository signature is not valid UTF-8",
            )
        })?;
        let envelope = parse_signature_envelope(envelope_text, SignedObject::RepositoryIndex)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;

        let previous = self.load_verified_repository_if_present(
            request.repository_id,
            request.channel,
            request.mirror_id,
            request.now_seconds,
            &trusted_keys,
            true,
        )?;
        let sequence = read_repository_sequence_state(&self.data_root, request.channel);
        if sequence.present && !sequence.valid {
            return Err(PackageFacadeError::new(
                "repository_sequence_corrupt",
                "repository sequence state is corrupt and requires repair",
            ));
        }
        let policy = RepositoryVerificationPolicy::new(
            request.repository_id,
            request.channel,
            request.mirror_id,
            request.now_seconds,
        );
        let policy = previous.as_ref().map_or(policy, |repository| {
            policy.for_refresh_after(repository.sequence())
        });
        let repository = verify_repository_index_envelope_with_policy(
            &index_bytes,
            &envelope,
            &trusted_keys,
            &policy,
        )
        .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        let release_sequence = maximum_release_sequence(&repository);
        if sequence.present && release_sequence < sequence.maximum {
            return Err(PackageFacadeError::new(
                "rollback_rejected",
                "repository package release sequence is older than the accepted state",
            ));
        }

        self.publish_verified_repository(
            request.channel,
            &index_bytes,
            &signature_bytes,
            &sequence,
            release_sequence,
        )?;
        Ok(RepositoryRefreshResult {
            sequence: repository.sequence(),
            package_count: repository.packages().len(),
        })
    }

    /// Loads and re-verifies the active repository cache before package resolution.
    pub fn load_verified_repository(
        &self,
        repository_id: &str,
        channel: &str,
        mirror_id: &str,
        now_seconds: u64,
    ) -> Result<RepositoryIndex, PackageFacadeError> {
        let trusted_keys = self.read_production_trusted_keys()?;
        self.load_verified_repository_if_present(
            repository_id,
            channel,
            mirror_id,
            now_seconds,
            &trusted_keys,
            false,
        )?
        .ok_or_else(|| {
            PackageFacadeError::new(
                "repository_unavailable",
                "no verified repository cache exists",
            )
        })
    }

    /// Downloads, verifies, stages, activates, and records one exact dependency plan.
    pub fn install_or_update<T: RepositoryTransport>(
        &self,
        repository_id: &str,
        channel: &str,
        mirror_id: &str,
        now_seconds: u64,
        request: PackageInstallRequest<'_>,
        transport: &mut T,
    ) -> Result<PackageInstallResult, PackageFacadeError> {
        let repository =
            self.load_verified_repository(repository_id, channel, mirror_id, now_seconds)?;
        let mut executor = LocalPackageTransactionExecutor;
        self.install_or_update_from_repository(&repository, request, transport, &mut executor)
    }

    /// Executes an already-verified repository plan through an injected filesystem/elevation seam.
    ///
    /// This is public for deterministic package-core tests and for the narrow deployer boundary;
    /// regular Control callers should use [`Self::install_or_update`].
    pub fn install_or_update_from_repository<
        T: RepositoryTransport,
        E: PackageTransactionExecutor,
    >(
        &self,
        repository: &RepositoryIndex,
        request: PackageInstallRequest<'_>,
        transport: &mut T,
        executor: &mut E,
    ) -> Result<PackageInstallResult, PackageFacadeError> {
        let transaction_id = validated_transaction_id(request.transaction_id)?;
        let plan = resolve_repository_plan(repository, request.requested_ids, &self.architecture)?;
        let trusted_keys = self.read_production_trusted_keys()?;
        let package_root = self.package_root();
        fs::create_dir_all(&package_root).map_err(|error| {
            PackageFacadeError::new("io_error", format!("package root creation failed: {error}"))
        })?;
        self.write_transaction_state(&transaction_id, TransactionState::Prepared)?;

        let mut activation_started = false;
        let result = (|| {
            for entry in plan.entries() {
                let cached_archive = self.data_root.join("downloads").join(format!(
                    "{}-{}.fcpkg",
                    entry.id(),
                    entry.version()
                ));
                let archive = if cached_archive.is_file() {
                    read_bounded_file(&cached_archive, MAXIMUM_ARCHIVE_BYTES)?
                } else {
                    transport.fetch(entry.download_url(), MAXIMUM_ARCHIVE_BYTES)?
                };
                ensure_bound(&archive, MAXIMUM_ARCHIVE_BYTES, "package archive")?;
                if sha256_digest(&archive) != *entry.sha256() {
                    return Err(PackageFacadeError::new(
                        "archive_digest_mismatch",
                        "downloaded package archive differs from verified repository metadata",
                    ));
                }
                let archive_path = self.write_cached_archive(&transaction_id, entry, &archive)?;
                self.write_transaction_state(&transaction_id, TransactionState::Staged)?;
                let staged =
                    executor.stage(&archive_path, &package_root, &transaction_id, &trusted_keys)?;
                self.write_transaction_state(&transaction_id, TransactionState::Activating)?;
                activation_started = true;
                executor.activate(&staged, &package_root, &trusted_keys)?;
            }
            self.write_transaction_state(&transaction_id, TransactionState::Committed)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.cleanup_transaction_artifacts(&transaction_id);
                Ok(PackageInstallResult {
                    transaction_id,
                    plan,
                })
            }
            Err(error) => {
                let rollback = if activation_started {
                    executor.rollback(&package_root, &transaction_id)
                } else {
                    Ok(())
                };
                self.cleanup_transaction_artifacts(&transaction_id);
                if let Err(rollback_error) = rollback {
                    return Err(PackageFacadeError::new(
                        "recovery_required",
                        format!("{error}; rollback also failed: {rollback_error}"),
                    ));
                }
                Err(error)
            }
        }
    }

    /// Changes enabled/disabled state through the checked lockfile owner.
    pub fn set_package_state(
        &self,
        package_id: &str,
        state: crate::PackageLifecycleState,
    ) -> Result<(), PackageFacadeError> {
        crate::set_installed_package_state(self.package_root(), package_id, state)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))
    }

    /// Removes a package only after the existing reverse-dependency checks have passed.
    pub fn remove_package(&self, package_id: &str) -> Result<(), PackageFacadeError> {
        crate::mark_installed_package_for_removal(self.package_root(), package_id)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        crate::finalize_installed_package_removal(self.package_root(), package_id)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))
    }

    /// Stages and activates a complete Core release, preserving the previous known-good package
    /// when deployment-state publication fails.
    ///
    /// # Errors
    ///
    /// Returns a verification, compatibility, activation, or recovery error without claiming a
    /// successful health check.
    pub fn activate_core_update(
        &self,
        archive: &Path,
        transaction_id: &str,
        keyring_path: &Path,
        channel: &str,
    ) -> Result<CoreUpdateActivation, PackageFacadeError> {
        let transaction_id = validated_transaction_id(transaction_id)?;
        let owner = update::read_update_owner(&self.data_root).map_err(update_error)?;
        if owner != update::Fcitx5UpdateOwner::Builtin {
            return Err(PackageFacadeError::new(
                "update_owner",
                "Core update is owned by an external package manager",
            ));
        }
        let trusted_keys = read_trusted_keyring(keyring_path)?;
        let package_root = self.package_root();
        let old_lock = read_installed_lockfile(&package_root)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        let staged =
            stage_verified_archive(archive, &package_root, &transaction_id, &trusted_keys)?;
        let manifest_bytes =
            read_bounded_file(staged.join("manifest.json"), MAX_MANIFEST_BYTES as u64)?;
        let manifest_text = std::str::from_utf8(&manifest_bytes).map_err(|_| {
            PackageFacadeError::new("invalid_manifest", "staged manifest is not UTF-8")
        })?;
        let manifest = parse_manifest(manifest_text)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        if manifest.package_type() != &PackageType::Core {
            return Err(PackageFacadeError::new(
                "invalid_manifest",
                "updater accepts only a complete Core release package",
            ));
        }
        activate_verified_staged_payload(&staged, &package_root, &trusted_keys)?;
        if let Err(error) =
            update::begin_activation(&self.data_root, channel, manifest.version(), owner)
        {
            let Some(previous) = old_lock.iter().find(|entry| entry.id() == manifest.id()) else {
                return Err(PackageFacadeError::new(
                    "recovery_required",
                    format!("deployment state publication failed after activation: {error}"),
                ));
            };
            activate_installed_version_for_rollback(
                &package_root,
                previous.id().as_str(),
                previous.version(),
                &trusted_keys,
            )
            .map_err(|rollback| {
                PackageFacadeError::new(
                    "recovery_required",
                    format!(
                        "deployment state publication failed: {error}; rollback failed: {rollback}"
                    ),
                )
            })?;
            return Err(update_error(error));
        }
        Ok(CoreUpdateActivation {
            version: manifest.version().to_owned(),
        })
    }

    /// Marks the active Core release healthy after its external health check passes.
    ///
    /// # Errors
    ///
    /// Returns an error unless the channel has a matching pending activation.
    pub fn mark_core_update_healthy(&self, channel: &str) -> Result<(), PackageFacadeError> {
        update::mark_current_healthy(&self.data_root, channel).map_err(update_error)
    }

    /// Rolls the complete Core package back to its previous known-good release.
    ///
    /// # Errors
    ///
    /// Returns an error if no previous release exists or its verified package payload cannot be
    /// activated.
    pub fn rollback_core_update(
        &self,
        channel: &str,
        package_id: &str,
        keyring_path: &Path,
    ) -> Result<String, PackageFacadeError> {
        let trusted_keys = read_trusted_keyring(keyring_path)?;
        let target = update::rollback_target(&self.data_root, channel).map_err(update_error)?;
        activate_installed_version_for_rollback(
            self.package_root(),
            package_id,
            &target,
            &trusted_keys,
        )
        .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        update::finish_rollback(&self.data_root, channel).map_err(update_error)?;
        Ok(target)
    }

    /// Removes an unneeded previous-known-good package version after a healthy deployment.
    ///
    /// # Errors
    ///
    /// Returns the existing deployment-state error when cleanup is not safe.
    pub fn cleanup_previous_core_update(
        &self,
        channel: &str,
        package_id: &str,
    ) -> Result<(), PackageFacadeError> {
        update::cleanup_previous_known_good(&self.data_root, channel, package_id)
            .map_err(update_error)
    }

    /// Reads the production keyring, checks installed ownership, and safely cleans abandoned work.
    pub fn repair(&self) -> Result<PackageRepairReport, PackageFacadeError> {
        let trusted_keys = self.read_production_trusted_keys()?;
        let package_root = self.package_root();
        let verification = match verify_installed_packages_for_repair(&package_root, &trusted_keys)
        {
            Ok(()) => PackageRepairVerification::Verified,
            Err(error) => PackageRepairVerification::Failed {
                code: error.code().to_owned(),
            },
        };
        let mut actions = self.reconcile_transaction_journals()?;
        actions.extend(self.inspect_unowned_package_directories()?);
        Ok(PackageRepairReport {
            verification,
            actions,
        })
    }

    /// Repairs or resets the signed repository anti-rollback state.
    pub fn repair_repository_sequence(
        &self,
        channel: &str,
    ) -> Result<&'static str, PackageFacadeError> {
        let trusted_keys = self.read_production_trusted_keys()?;
        Ok(crate::repair_repository_sequence_state_for_repair(
            &self.data_root,
            self.repository_index_path(),
            self.repository_signature_path(),
            &trusted_keys,
            channel,
        ))
    }

    /// Reads the trusted keys only from the installed product keyring.
    pub fn read_production_trusted_keys(&self) -> Result<Vec<TrustedKey>, PackageFacadeError> {
        read_trusted_keyring(self.install_root.join("security/trusted-keys.json"))
    }

    fn package_root(&self) -> PathBuf {
        self.data_root.join("packages")
    }

    fn repository_index_path(&self) -> PathBuf {
        self.data_root.join("repository").join("index.json")
    }

    fn repository_signature_path(&self) -> PathBuf {
        self.data_root.join("repository").join("index.sig.json")
    }

    fn load_verified_repository_if_present(
        &self,
        repository_id: &str,
        channel: &str,
        mirror_id: &str,
        now_seconds: u64,
        trusted_keys: &[TrustedKey],
        accept_expired_refresh_reference: bool,
    ) -> Result<Option<RepositoryIndex>, PackageFacadeError> {
        let index_path = self.repository_index_path();
        let signature_path = self.repository_signature_path();
        if !index_path.exists() && !signature_path.exists() {
            return Ok(None);
        }
        if !index_path.is_file() || !signature_path.is_file() {
            return Err(PackageFacadeError::new(
                "invalid_repository",
                "repository cache is incomplete",
            ));
        }
        let index = read_bounded_file(&index_path, MAX_MANIFEST_BYTES as u64)?;
        let signature = read_bounded_file(&signature_path, MAXIMUM_REPOSITORY_SIGNATURE_BYTES)?;
        let signature_text = std::str::from_utf8(&signature).map_err(|_| {
            PackageFacadeError::new(
                "invalid_signature",
                "cached repository signature is not UTF-8",
            )
        })?;
        let envelope = parse_signature_envelope(signature_text, SignedObject::RepositoryIndex)
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        let policy =
            RepositoryVerificationPolicy::new(repository_id, channel, mirror_id, now_seconds);
        let policy = if accept_expired_refresh_reference {
            policy.for_refresh_reference()
        } else {
            policy
        };
        let repository =
            verify_repository_index_envelope_with_policy(&index, &envelope, trusted_keys, &policy)
                .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        let sequence = read_repository_sequence_state(&self.data_root, channel);
        if !sequence.present {
            return Err(PackageFacadeError::new(
                "repository_sequence_missing",
                "verified repository cache has no anti-rollback state",
            ));
        }
        if !sequence.valid {
            return Err(PackageFacadeError::new(
                "repository_sequence_corrupt",
                "repository sequence state is corrupt and requires repair",
            ));
        }
        if maximum_release_sequence(&repository) != sequence.maximum {
            return Err(PackageFacadeError::new(
                "rollback_rejected",
                "repository cache does not match the accepted release sequence",
            ));
        }
        Ok(Some(repository))
    }

    fn publish_verified_repository(
        &self,
        channel: &str,
        index: &[u8],
        signature: &[u8],
        previous_sequence: &RepositorySequenceState,
        release_sequence: u64,
    ) -> Result<(), PackageFacadeError> {
        write_repository_sequence_state(&self.data_root, channel, release_sequence).map_err(
            |error| {
                PackageFacadeError::new(
                    "io_error",
                    format!("repository sequence publication failed: {error}"),
                )
            },
        )?;
        let write_cache = write_atomic_file(&self.repository_signature_path(), signature)
            .and_then(|()| write_atomic_file(&self.repository_index_path(), index))
            .map_err(|error| {
                PackageFacadeError::new(
                    "io_error",
                    format!("verified repository cache publication failed: {error}"),
                )
            });
        if let Err(error) = write_cache {
            restore_repository_sequence(&self.data_root, channel, previous_sequence)?;
            return Err(error);
        }
        Ok(())
    }

    fn write_cached_archive(
        &self,
        transaction_id: &str,
        entry: &RepositoryEntry,
        archive: &[u8],
    ) -> Result<PathBuf, PackageFacadeError> {
        let cache = self
            .package_root()
            .join("archive-cache")
            .join(format!("{transaction_id}-{}.fcpkg", entry.id()));
        write_new_file(&cache, archive).map_err(|error| {
            PackageFacadeError::new(
                "io_error",
                format!("verified archive cache write failed: {error}"),
            )
        })?;
        Ok(cache)
    }

    fn transaction_state_path(&self, transaction_id: &str) -> PathBuf {
        self.package_root()
            .join("transactions")
            .join(format!("{transaction_id}.state"))
    }

    fn write_transaction_state(
        &self,
        transaction_id: &str,
        state: TransactionState,
    ) -> Result<(), PackageFacadeError> {
        let path = self.transaction_state_path(transaction_id);
        if path.exists() && state == TransactionState::Prepared {
            return Err(PackageFacadeError::new(
                "transaction_exists",
                "package transaction is already in progress",
            ));
        }
        let text = format!(
            "format_version=1\ntransaction={transaction_id}\nstate={}\n",
            state.as_str()
        );
        write_atomic_file(&path, text.as_bytes()).map_err(|error| {
            PackageFacadeError::new(
                "io_error",
                format!("transaction state write failed: {error}"),
            )
        })
    }

    fn cleanup_transaction_artifacts(&self, transaction_id: &str) {
        let package_root = self.package_root();
        let _ = fs::remove_dir_all(package_root.join("staging").join(transaction_id));
        let _ = fs::remove_file(self.transaction_state_path(transaction_id));
        let cache = package_root.join("archive-cache");
        if let Ok(entries) = fs::read_dir(cache) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name
                    .to_string_lossy()
                    .starts_with(&format!("{transaction_id}-"))
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    fn reconcile_transaction_journals(
        &self,
    ) -> Result<Vec<PackageRepairAction>, PackageFacadeError> {
        let root = self.package_root().join("transactions");
        if !root.exists() {
            return Ok(Vec::new());
        }
        let entries = fs::read_dir(root).map_err(|error| {
            PackageFacadeError::new(
                "io_error",
                format!("transaction inspection failed: {error}"),
            )
        })?;
        let mut actions = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                PackageFacadeError::new(
                    "io_error",
                    format!("transaction inspection failed: {error}"),
                )
            })?;
            let path = entry.path();
            let Some(transaction_id) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .filter(|id| PackageId::parse(id).is_ok())
                .map(str::to_owned)
            else {
                continue;
            };
            let state = read_transaction_state(path, &transaction_id)?;
            match state {
                TransactionState::Prepared | TransactionState::Staged => {
                    self.cleanup_transaction_artifacts(&transaction_id);
                    actions.push(PackageRepairAction::DiscardedIncompleteTransaction {
                        transaction_id: transaction_id.to_owned(),
                    });
                }
                TransactionState::Activating => {
                    actions.push(PackageRepairAction::RecoveryRequired {
                        transaction_id: transaction_id.to_owned(),
                    });
                }
                TransactionState::Committed => {
                    self.cleanup_transaction_artifacts(&transaction_id);
                    actions.push(PackageRepairAction::FinalizedCommittedTransaction {
                        transaction_id: transaction_id.to_owned(),
                    });
                }
            }
        }
        Ok(actions)
    }

    fn inspect_unowned_package_directories(
        &self,
    ) -> Result<Vec<PackageRepairAction>, PackageFacadeError> {
        let lock = read_installed_lockfile(self.package_root())
            .map_err(|error| PackageFacadeError::new(error.code(), error.to_string()))?;
        let owned = lock
            .iter()
            .map(|entry| entry.id().as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let mut actions = Vec::new();
        for root in ["versions", "manifests"] {
            let path = self.package_root().join(root);
            if !path.exists() {
                continue;
            }
            for entry in fs::read_dir(path).map_err(|error| {
                PackageFacadeError::new(
                    "io_error",
                    format!("payload ownership inspection failed: {error}"),
                )
            })? {
                let entry = entry.map_err(|error| {
                    PackageFacadeError::new(
                        "io_error",
                        format!("payload ownership inspection failed: {error}"),
                    )
                })?;
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().is_dir() && !owned.contains(&name) {
                    actions.push(PackageRepairAction::UnownedPackageDirectory {
                        relative_path: format!("{root}/{name}"),
                    });
                }
            }
        }
        Ok(actions)
    }
}

/// Inputs that bind repository refresh to one trusted channel and mirror identity.
#[derive(Clone, Copy, Debug)]
pub struct RepositoryRefreshRequest<'a> {
    /// HTTPS endpoint for strict JSON repository metadata.
    pub index_url: &'a str,
    /// HTTPS endpoint for the repository signature envelope.
    pub signature_url: &'a str,
    /// Expected immutable repository identity.
    pub repository_id: &'a str,
    /// Expected release channel.
    pub channel: &'a str,
    /// Expected mirror identity; mirrors are not trust roots.
    pub mirror_id: &'a str,
    /// Caller-supplied clock for deterministic verification.
    pub now_seconds: u64,
}

/// Summary of an atomically published repository refresh.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryRefreshResult {
    sequence: u64,
    package_count: usize,
}

impl RepositoryRefreshResult {
    /// Accepted repository sequence.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Number of verified repository package records.
    #[must_use]
    pub fn package_count(&self) -> usize {
        self.package_count
    }
}

/// An exact dependency-ordered package plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryPlan {
    entries: Vec<RepositoryEntry>,
}

impl RepositoryPlan {
    /// Entries are ordered with exact dependencies before their dependents.
    #[must_use]
    pub fn entries(&self) -> &[RepositoryEntry] {
        &self.entries
    }
}

/// Resolves package ids against the signed repository's exact versions and target architecture.
pub fn resolve_repository_plan(
    repository: &RepositoryIndex,
    requested_ids: &[&str],
    architecture: &str,
) -> Result<RepositoryPlan, PackageFacadeError> {
    if requested_ids.is_empty() {
        return Err(PackageFacadeError::new(
            "invalid_request",
            "package request must not be empty",
        ));
    }
    let mut selected = BTreeMap::<String, RepositoryEntry>::new();
    let mut visiting = BTreeSet::<String>::new();
    let mut ordered = Vec::new();
    for requested in requested_ids {
        resolve_repository_entry(
            repository,
            requested,
            None,
            architecture,
            &mut selected,
            &mut visiting,
            &mut ordered,
        )?;
    }
    Ok(RepositoryPlan { entries: ordered })
}

/// Input for one transactional install or update request.
#[derive(Clone, Copy, Debug)]
pub struct PackageInstallRequest<'a> {
    /// User-selected package ids.
    pub requested_ids: &'a [&'a str],
    /// Caller-chosen stable transaction id.
    pub transaction_id: &'a str,
}

/// Successful package activation and the exact plan that was applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInstallResult {
    transaction_id: String,
    plan: RepositoryPlan,
}

/// A Core release that has been activated and is awaiting its health check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreUpdateActivation {
    version: String,
}

impl CoreUpdateActivation {
    /// Version that is pending the external Core health check.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl PackageInstallResult {
    /// Completed transaction id.
    #[must_use]
    pub fn transaction_id(&self) -> &str {
        &self.transaction_id
    }

    /// Activated exact dependency plan.
    #[must_use]
    pub fn plan(&self) -> &RepositoryPlan {
        &self.plan
    }
}

/// Filesystem/elevation seam used only around existing archive staging and activation owners.
pub trait PackageTransactionExecutor {
    /// Validates and stages a verified archive.
    fn stage(
        &mut self,
        archive_path: &Path,
        package_root: &Path,
        transaction_id: &str,
        trusted_keys: &[TrustedKey],
    ) -> Result<PathBuf, PackageFacadeError>;

    /// Activates one staged package through the existing owner.
    fn activate(
        &mut self,
        staged_root: &Path,
        package_root: &Path,
        trusted_keys: &[TrustedKey],
    ) -> Result<(), PackageFacadeError>;

    /// Restores deterministic safe state after a stage or activation fault.
    fn rollback(
        &mut self,
        package_root: &Path,
        transaction_id: &str,
    ) -> Result<(), PackageFacadeError>;
}

struct LocalPackageTransactionExecutor;

impl PackageTransactionExecutor for LocalPackageTransactionExecutor {
    fn stage(
        &mut self,
        archive_path: &Path,
        package_root: &Path,
        transaction_id: &str,
        trusted_keys: &[TrustedKey],
    ) -> Result<PathBuf, PackageFacadeError> {
        stage_verified_archive(archive_path, package_root, transaction_id, trusted_keys)
    }

    fn activate(
        &mut self,
        staged_root: &Path,
        package_root: &Path,
        trusted_keys: &[TrustedKey],
    ) -> Result<(), PackageFacadeError> {
        activate_verified_staged_payload(staged_root, package_root, trusted_keys)
    }

    fn rollback(
        &mut self,
        package_root: &Path,
        transaction_id: &str,
    ) -> Result<(), PackageFacadeError> {
        fs::remove_dir_all(package_root.join("staging").join(transaction_id))
            .or_else(|error| {
                (error.kind() == std::io::ErrorKind::NotFound)
                    .then_some(())
                    .ok_or(error)
            })
            .map_err(|error| {
                PackageFacadeError::new("io_error", format!("staging rollback failed: {error}"))
            })
    }
}

/// Verification status returned by [`PackageCoreFacade::repair`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageRepairVerification {
    /// Lockfile, manifests, signatures, and owned payloads verified.
    Verified,
    /// Verification failed; the code identifies the first deterministic failing check.
    Failed {
        /// Stable error code from the existing repair owner.
        code: String,
    },
}

/// A safe action taken or surfaced by package repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageRepairAction {
    /// Staging and archive-cache data was abandoned before activation and has been discarded.
    DiscardedIncompleteTransaction {
        /// Journal transaction id.
        transaction_id: String,
    },
    /// Activation status is ambiguous and needs an explicit follow-up repair after verification.
    RecoveryRequired {
        /// Journal transaction id.
        transaction_id: String,
    },
    /// An already committed transaction only had stale journal/cache residue.
    FinalizedCommittedTransaction {
        /// Journal transaction id.
        transaction_id: String,
    },
    /// A version or manifest directory has no lockfile owner and is not deleted implicitly.
    UnownedPackageDirectory {
        /// Package-root-relative path requiring review.
        relative_path: String,
    },
}

/// Typed repair outcome without unrelated registration side effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRepairReport {
    verification: PackageRepairVerification,
    actions: Vec<PackageRepairAction>,
}

impl PackageRepairReport {
    /// Integrity verification result.
    #[must_use]
    pub fn verification(&self) -> &PackageRepairVerification {
        &self.verification
    }

    /// Safe reconciliation actions taken or follow-up actions required.
    #[must_use]
    pub fn actions(&self) -> &[PackageRepairAction] {
        &self.actions
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransactionState {
    Prepared,
    Staged,
    Activating,
    Committed,
}

impl TransactionState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Staged => "staged",
            Self::Activating => "activating",
            Self::Committed => "committed",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "prepared" => Some(Self::Prepared),
            "staged" => Some(Self::Staged),
            "activating" => Some(Self::Activating),
            "committed" => Some(Self::Committed),
            _ => None,
        }
    }
}

fn validate_refresh_request(
    request: &RepositoryRefreshRequest<'_>,
) -> Result<(), PackageFacadeError> {
    if !is_credential_free_https_url(request.index_url)
        || !is_credential_free_https_url(request.signature_url)
    {
        return Err(PackageFacadeError::new(
            "invalid_repository_url",
            "repository endpoint must be credential-free HTTPS without a fragment",
        ));
    }
    if !valid_repository_token(request.repository_id)
        || !valid_repository_token(request.channel)
        || !valid_repository_token(request.mirror_id)
    {
        return Err(PackageFacadeError::new(
            "invalid_repository",
            "repository identity is invalid",
        ));
    }
    Ok(())
}

fn is_credential_free_https_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?']).next().unwrap_or_default();
    !authority.is_empty()
        && value.len() <= 2048
        && !value.contains(['@', '#', '\\'])
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

fn valid_repository_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn update_error(message: String) -> PackageFacadeError {
    PackageFacadeError::new("update_state", message)
}

fn maximum_release_sequence(repository: &RepositoryIndex) -> u64 {
    repository
        .packages()
        .iter()
        .map(RepositoryEntry::release_sequence)
        .max()
        .unwrap_or(0)
}

fn resolve_repository_entry(
    repository: &RepositoryIndex,
    package_id: &str,
    expected_version: Option<&str>,
    architecture: &str,
    selected: &mut BTreeMap<String, RepositoryEntry>,
    visiting: &mut BTreeSet<String>,
    ordered: &mut Vec<RepositoryEntry>,
) -> Result<(), PackageFacadeError> {
    if let Some(entry) = selected.get(package_id) {
        if expected_version.is_none_or(|version| entry.version() == version) {
            return Ok(());
        }
        return Err(PackageFacadeError::new(
            "dependency_conflict",
            "repository dependencies require incompatible exact versions",
        ));
    }
    if !visiting.insert(package_id.to_owned()) {
        return Err(PackageFacadeError::new(
            "dependency_cycle",
            "repository dependency graph contains a cycle",
        ));
    }
    let entry = select_repository_entry(repository, package_id, architecture).ok_or_else(|| {
        PackageFacadeError::new("package_unavailable", "requested package is unavailable")
    })?;
    if expected_version.is_some_and(|version| entry.version() != version) {
        return Err(PackageFacadeError::new(
            "dependency_conflict",
            "repository dependency requires an unavailable exact version",
        ));
    }
    for dependency in entry.dependencies() {
        resolve_repository_entry(
            repository,
            dependency.id(),
            Some(dependency.version()),
            architecture,
            selected,
            visiting,
            ordered,
        )?;
    }
    visiting.remove(package_id);
    selected.insert(package_id.to_owned(), entry.clone());
    ordered.push(entry.clone());
    Ok(())
}

fn select_repository_entry<'a>(
    repository: &'a RepositoryIndex,
    package_id: &str,
    architecture: &str,
) -> Option<&'a RepositoryEntry> {
    repository
        .packages()
        .iter()
        .find(|entry| entry.id() == package_id && entry.architecture() == architecture)
        .or_else(|| {
            repository
                .packages()
                .iter()
                .find(|entry| entry.id() == package_id && entry.architecture() == "any")
        })
}

fn validated_transaction_id(transaction_id: &str) -> Result<String, PackageFacadeError> {
    PackageId::parse(transaction_id)
        .map(|id| id.as_str().to_owned())
        .map_err(|_| {
            PackageFacadeError::new(
                "invalid_transaction",
                "transaction id is not a safe package token",
            )
        })
}

fn ensure_bound(bytes: &[u8], maximum: u64, label: &str) -> Result<(), PackageFacadeError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(PackageFacadeError::new(
            "resource_limit",
            format!("{label} exceeds its resource budget"),
        ));
    }
    Ok(())
}

fn read_bounded_file(path: impl AsRef<Path>, maximum: u64) -> Result<Vec<u8>, PackageFacadeError> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|_| {
        PackageFacadeError::new("io_error", "required package file is missing or unreadable")
    })?;
    if metadata.len() > maximum {
        return Err(PackageFacadeError::new(
            "resource_limit",
            "required package file exceeds its resource budget",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|_| PackageFacadeError::new("io_error", "required package file is unreadable"))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != metadata.len() {
        return Err(PackageFacadeError::new(
            "io_error",
            "required package file changed while it was read",
        ));
    }
    Ok(bytes)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "package cache path is invalid",
        )
    })?;
    fs::create_dir_all(parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()
}

fn write_atomic_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "package state path is invalid",
        )
    })?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("new");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let write = (|| {
        file.write_all(bytes)?;
        file.flush()?;
        crate::win32_fs_adapter::replace_file(&temporary, path)
    })();
    if write.is_err() {
        let _ = fs::remove_file(temporary);
    }
    write
}

fn restore_repository_sequence(
    data_root: &Path,
    channel: &str,
    previous: &RepositorySequenceState,
) -> Result<(), PackageFacadeError> {
    let path = data_root
        .join("repository")
        .join(format!("sequence-{channel}.json"));
    if !previous.present {
        fs::remove_file(path)
            .or_else(|error| {
                (error.kind() == std::io::ErrorKind::NotFound)
                    .then_some(())
                    .ok_or(error)
            })
            .map_err(|error| {
                PackageFacadeError::new(
                    "recovery_required",
                    format!("repository sequence rollback failed: {error}"),
                )
            })
    } else if previous.valid {
        write_repository_sequence_state(data_root, channel, previous.maximum).map_err(|error| {
            PackageFacadeError::new(
                "recovery_required",
                format!("repository sequence rollback failed: {error}"),
            )
        })
    } else {
        Err(PackageFacadeError::new(
            "recovery_required",
            "repository sequence was already corrupt",
        ))
    }
}

fn read_transaction_state(
    path: PathBuf,
    transaction_id: &str,
) -> Result<TransactionState, PackageFacadeError> {
    let bytes = read_bounded_file(path, MAXIMUM_TRANSACTION_STATE_BYTES)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        PackageFacadeError::new("invalid_transaction", "transaction journal is not UTF-8")
    })?;
    let expected = format!("format_version=1\ntransaction={transaction_id}\nstate=");
    let state = text
        .strip_prefix(&expected)
        .and_then(|value| value.strip_suffix('\n'))
        .and_then(TransactionState::parse)
        .ok_or_else(|| {
            PackageFacadeError::new(
                "invalid_transaction",
                "transaction journal format is invalid",
            )
        })?;
    Ok(state)
}
