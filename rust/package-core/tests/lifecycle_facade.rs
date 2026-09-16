#![cfg(windows)]
#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use fcitx5_package_core::{
    parse_repository_index_with_policy, resolve_repository_plan, sha256_digest, PackageCoreFacade,
    PackageFacadeError, PackageInstallRequest, PackageRepairAction, PackageTransactionExecutor,
    RepositoryRefreshRequest, RepositoryTransport, RepositoryVerificationPolicy, TrustedKey,
};

const KEYRING: &str = include_str!("../../../security/trusted-keys.template.json");

struct FixtureRoot(PathBuf);

impl FixtureRoot {
    fn new() -> Self {
        let unique = format!(
            "fcitx5-package-facade-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("fixture clock should be after epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("fixture root should create");
        Self(path)
    }

    fn install_root(&self) -> PathBuf {
        self.0.join("install")
    }

    fn data_root(&self) -> PathBuf {
        self.0.join("data")
    }

    fn facade(&self) -> PackageCoreFacade {
        let install_root = self.install_root();
        let data_root = self.data_root();
        std::fs::create_dir_all(install_root.join("security"))
            .expect("security root should create");
        std::fs::create_dir_all(&data_root).expect("data root should create");
        std::fs::write(install_root.join("security/trusted-keys.json"), KEYRING)
            .expect("keyring should write");
        PackageCoreFacade::new(install_root, data_root, "x64")
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct FixtureTransport {
    responses: VecDeque<Vec<u8>>,
    calls: usize,
}

impl FixtureTransport {
    fn with(responses: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
            calls: 0,
        }
    }
}

impl RepositoryTransport for FixtureTransport {
    fn fetch(&mut self, _url: &str, _maximum_bytes: u64) -> Result<Vec<u8>, PackageFacadeError> {
        self.calls += 1;
        self.responses
            .pop_front()
            .ok_or_else(|| PackageFacadeError::new("network_error", "fixture response is missing"))
    }
}

struct FailingExecutor {
    rollback_called: bool,
    failure_code: &'static str,
}

impl FailingExecutor {
    fn with_failure(failure_code: &'static str) -> Self {
        Self {
            rollback_called: false,
            failure_code,
        }
    }
}

impl Default for FailingExecutor {
    fn default() -> Self {
        Self::with_failure("activation_failed")
    }
}

impl PackageTransactionExecutor for FailingExecutor {
    fn stage(
        &mut self,
        _archive_path: &Path,
        package_root: &Path,
        transaction_id: &str,
        _trusted_keys: &[TrustedKey],
    ) -> Result<PathBuf, PackageFacadeError> {
        let staged = package_root.join("staging").join(transaction_id);
        std::fs::create_dir_all(&staged).expect("fixture staged directory should create");
        Ok(staged)
    }

    fn activate(
        &mut self,
        _staged_root: &Path,
        _package_root: &Path,
        _trusted_keys: &[TrustedKey],
    ) -> Result<(), PackageFacadeError> {
        Err(PackageFacadeError::new(
            self.failure_code,
            "fixture activation failed",
        ))
    }

    fn rollback(
        &mut self,
        _package_root: &Path,
        _transaction_id: &str,
    ) -> Result<(), PackageFacadeError> {
        self.rollback_called = true;
        Ok(())
    }
}

#[derive(Default)]
struct RecordingExecutor {
    stages: usize,
    activations: usize,
}

impl PackageTransactionExecutor for RecordingExecutor {
    fn stage(
        &mut self,
        _archive_path: &Path,
        package_root: &Path,
        transaction_id: &str,
        _trusted_keys: &[TrustedKey],
    ) -> Result<PathBuf, PackageFacadeError> {
        self.stages += 1;
        let staged = package_root.join("staging").join(transaction_id);
        std::fs::create_dir_all(&staged).expect("fixture staged directory should create");
        Ok(staged)
    }

    fn activate(
        &mut self,
        _staged_root: &Path,
        _package_root: &Path,
        _trusted_keys: &[TrustedKey],
    ) -> Result<(), PackageFacadeError> {
        self.activations += 1;
        Ok(())
    }

    fn rollback(
        &mut self,
        _package_root: &Path,
        _transaction_id: &str,
    ) -> Result<(), PackageFacadeError> {
        panic!("successful transaction must not roll back")
    }
}

struct MutatingFailingExecutor {
    activations: usize,
    rollback_called: bool,
    fail_after: usize,
}

impl Default for MutatingFailingExecutor {
    fn default() -> Self {
        Self {
            activations: 0,
            rollback_called: false,
            fail_after: 2,
        }
    }
}

impl PackageTransactionExecutor for MutatingFailingExecutor {
    fn stage(
        &mut self,
        _archive_path: &Path,
        package_root: &Path,
        transaction_id: &str,
        _trusted_keys: &[TrustedKey],
    ) -> Result<PathBuf, PackageFacadeError> {
        let staged = package_root.join("staging").join(transaction_id);
        std::fs::create_dir_all(&staged).expect("fixture staged directory should create");
        Ok(staged)
    }

    fn activate(
        &mut self,
        _staged_root: &Path,
        package_root: &Path,
        _trusted_keys: &[TrustedKey],
    ) -> Result<(), PackageFacadeError> {
        self.activations += 1;
        let version = package_root
            .join("versions")
            .join("partial-addon")
            .join(format!("activation-{}", self.activations));
        std::fs::create_dir_all(&version).expect("fixture version directory should create");
        std::fs::write(version.join("payload.txt"), b"partial activation")
            .expect("fixture payload should write");
        std::fs::write(package_root.join("packages.lock"), b"partial-lock")
            .expect("fixture lock should write");
        if self.activations >= self.fail_after {
            Err(PackageFacadeError::new(
                "activation_failed",
                "fixture failed after mutating installed state",
            ))
        } else {
            Ok(())
        }
    }

    fn rollback(
        &mut self,
        package_root: &Path,
        transaction_id: &str,
    ) -> Result<(), PackageFacadeError> {
        self.rollback_called = true;
        std::fs::remove_dir_all(package_root.join("staging").join(transaction_id))
            .or_else(|error| {
                (error.kind() == std::io::ErrorKind::NotFound)
                    .then_some(())
                    .ok_or(error)
            })
            .map_err(|error| PackageFacadeError::new("io_error", error.to_string()))
    }
}

fn repository(sequence: u64, expires_at: &str, dependencies: &str, addon_hash: &str) -> String {
    let data_hash = addon_hash;
    let targets = format!(
        "fcitx5-rime\t2.0.0\t2\tx64\t{addon_hash}\nrime-data\t1.0.0\t1\tany\t{data_hash}\n"
    );
    let targets_sha256 = sha256_digest(targets.as_bytes());
    format!(
        r#"{{"format_version":1,"repository_id":"fcitx5-windows-next","channel":"stable","mirror_id":"official","sequence":{sequence},"generated_at":"2026-08-17T00:00:00Z","expires_at":"{expires_at}","key_id":"official-2026-mldsa65","targets":{{"count":2,"sha256":"{}"}},"packages":[{{"id":"fcitx5-rime","title":"Rime","summary":"Rime input method","version":"2.0.0","release_sequence":2,"type":"addon","architecture":"x64","download_url":"https://packages.example.invalid/rime.fcpkg","sha256":"{addon_hash}","dependencies":{dependencies}}},{{"id":"rime-data","title":"Rime data","summary":"","version":"1.0.0","release_sequence":1,"type":"inputmethod-data","architecture":"any","download_url":"https://packages.example.invalid/rime-data.fcpkg","sha256":"{data_hash}","dependencies":[]}}]}}"#,
        targets_sha256.as_str(),
    )
}

fn repository_index() -> fcitx5_package_core::RepositoryIndex {
    let policy = RepositoryVerificationPolicy::new(
        "fcitx5-windows-next",
        "stable",
        "official",
        1_786_950_000,
    );
    parse_repository_index_with_policy(
        &repository(
            8,
            "2026-08-21T00:00:00Z",
            r#"[{"id":"rime-data","version":"1.0.0"}]"#,
            sha256_digest(&[0_u8; 12]).as_str(),
        ),
        &policy,
    )
    .expect("repository fixture should parse")
}

#[test]
fn refresh_rejects_http_credentials_and_fragments_before_transport() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    for endpoint in [
        "http://packages.example.invalid/index.json",
        "https://user@packages.example.invalid/index.json",
        "https://packages.example.invalid/index.json#fragment",
    ] {
        let mut transport = FixtureTransport::default();
        let error = facade
            .refresh_repository(
                RepositoryRefreshRequest {
                    index_url: endpoint,
                    signature_url: "https://packages.example.invalid/index.sig.json",
                    repository_id: "fcitx5-windows-next",
                    channel: "stable",
                    mirror_id: "official",
                    now_seconds: 1_786_950_000,
                },
                &mut transport,
            )
            .expect_err("unsafe endpoint must fail before transport");
        assert_eq!(error.code(), "invalid_repository_url");
        assert_eq!(transport.calls, 0);
    }
}

#[test]
fn repository_plan_is_dependency_ordered_and_exact() {
    let plan = resolve_repository_plan(&repository_index(), &["fcitx5-rime"], "x64")
        .expect("exact repository dependencies should resolve");
    assert_eq!(
        plan.entries()
            .iter()
            .map(|entry| (entry.id(), entry.version()))
            .collect::<Vec<_>>(),
        [("rime-data", "1.0.0"), ("fcitx5-rime", "2.0.0")]
    );

    let conflict = resolve_repository_plan(&repository_index(), &["fcitx5-rime", "missing"], "x64")
        .expect_err("missing dependency plan must not be silently reduced");
    assert_eq!(conflict.code(), "package_unavailable");
}

#[test]
fn bad_signature_leaves_previous_verified_cache_untouched() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    let cache = fixture.data_root().join("repository/index.json");
    std::fs::create_dir_all(cache.parent().expect("cache parent"))
        .expect("cache parent should create");
    std::fs::write(&cache, b"previous verified cache").expect("cache marker should write");
    let mut transport = FixtureTransport::with([
        repository_index().repository_id().as_bytes().to_vec(),
        b"not a signature envelope".to_vec(),
    ]);
    let error = facade
        .refresh_repository(
            RepositoryRefreshRequest {
                index_url: "https://packages.example.invalid/index.json",
                signature_url: "https://packages.example.invalid/index.sig.json",
                repository_id: "fcitx5-windows-next",
                channel: "stable",
                mirror_id: "official",
                now_seconds: 1_786_950_000,
            },
            &mut transport,
        )
        .expect_err("bad signature must fail");
    assert_eq!(error.code(), "invalid_signature");
    assert_eq!(
        std::fs::read(cache).expect("cache should remain"),
        b"previous verified cache"
    );
}

#[test]
fn transport_failure_leaves_verified_repository_cache_and_transaction_state_untouched() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    let repository_dir = fixture.data_root().join("repository");
    std::fs::create_dir_all(&repository_dir).expect("repository cache should create");
    let index = repository_dir.join("index.json");
    let signature = repository_dir.join("index.sig.json");
    let sequence = repository_dir.join("sequence-stable.json");
    std::fs::write(&index, b"last verified repository").expect("repository cache should write");
    std::fs::write(&signature, b"last verified signature")
        .expect("repository signature should write");
    std::fs::write(
        &sequence,
        b"format_version=1\nchannel=stable\nmax_release_sequence=8\n",
    )
    .expect("repository sequence should write");
    let repository_before = std::fs::read(&index).expect("repository cache should read");
    let signature_before = std::fs::read(&signature).expect("repository signature should read");
    let sequence_before = std::fs::read(&sequence).expect("repository sequence should read");

    let mut refresh_transport = FixtureTransport::default();
    let refresh_error = facade
        .refresh_repository(
            RepositoryRefreshRequest {
                index_url: "https://packages.example.invalid/index.json",
                signature_url: "https://packages.example.invalid/index.sig.json",
                repository_id: "fcitx5-windows-next",
                channel: "stable",
                mirror_id: "official",
                now_seconds: 1_786_950_000,
            },
            &mut refresh_transport,
        )
        .expect_err("refresh transport failure must be returned");
    assert_eq!(refresh_error.code(), "network_error");
    assert_eq!(refresh_transport.calls, 1);
    assert_eq!(
        std::fs::read(&index).expect("repository cache should remain"),
        repository_before
    );
    assert_eq!(
        std::fs::read(&signature).expect("repository signature should remain"),
        signature_before
    );
    assert_eq!(
        std::fs::read(&sequence).expect("repository sequence should remain"),
        sequence_before
    );

    let package_root = fixture.data_root().join("packages");
    let transaction = package_root.join("transactions").join("previous.state");
    let archive_cache = package_root
        .join("archive-cache")
        .join("previous-fcpkg.fcpkg");
    std::fs::create_dir_all(transaction.parent().expect("transaction parent"))
        .expect("transaction parent should create");
    std::fs::create_dir_all(archive_cache.parent().expect("archive cache parent"))
        .expect("archive cache parent should create");
    std::fs::write(
        &transaction,
        b"format_version=1\ntransaction=previous\nstate=committed\n",
    )
    .expect("previous transaction should write");
    std::fs::write(&archive_cache, b"last verified archive")
        .expect("previous archive cache should write");
    let transaction_before = std::fs::read(&transaction).expect("transaction should read");
    let archive_before = std::fs::read(&archive_cache).expect("archive cache should read");
    let request = PackageInstallRequest {
        requested_ids: &["fcitx5-rime"],
        transaction_id: "download-failure",
    };
    let mut download_transport = FixtureTransport::default();
    let mut executor = RecordingExecutor::default();
    let download_error = facade
        .install_or_update_from_repository(
            &repository_index(),
            request,
            &mut download_transport,
            &mut executor,
        )
        .expect_err("download transport failure must be returned");
    assert_eq!(download_error.code(), "network_error");
    assert_eq!(download_transport.calls, 1);
    assert_eq!(
        std::fs::read(&transaction).expect("previous transaction should remain"),
        transaction_before
    );
    assert_eq!(
        std::fs::read(&archive_cache).expect("previous archive cache should remain"),
        archive_before
    );
    assert!(
        !package_root
            .join("transactions")
            .join("download-failure.state")
            .exists(),
        "failed download must not leave its transaction journal"
    );
}

#[test]
fn keyring_expiry_and_rollback_policy_fail_closed() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    std::fs::write(
        fixture.install_root().join("security/trusted-keys.json"),
        "{}",
    )
    .expect("invalid keyring fixture should write");
    let keyring = facade
        .read_production_trusted_keys()
        .expect_err("malformed keyring must fail closed");
    assert_eq!(keyring.code(), "invalid_keyring");

    let expired = parse_repository_index_with_policy(
        &repository(
            8,
            "2026-08-17T00:00:00Z",
            r#"[{"id":"rime-data","version":"1.0.0"}]"#,
            sha256_digest(&[0_u8; 12]).as_str(),
        ),
        &RepositoryVerificationPolicy::new(
            "fcitx5-windows-next",
            "stable",
            "official",
            1_786_950_000,
        ),
    );
    assert!(
        expired.is_err(),
        "expired repository metadata must fail closed"
    );

    let frozen = parse_repository_index_with_policy(
        &repository(
            8,
            "2026-08-21T00:00:00Z",
            r#"[{"id":"rime-data","version":"1.0.0"}]"#,
            sha256_digest(&[0_u8; 12]).as_str(),
        ),
        &RepositoryVerificationPolicy::new(
            "fcitx5-windows-next",
            "stable",
            "official",
            1_786_950_000,
        )
        .for_refresh_after(8),
    );
    assert!(
        frozen.is_err(),
        "equal repository sequence must not refresh accepted metadata"
    );
}

#[test]
fn install_lock_fault_update_owner_and_repair_have_explicit_outcomes() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    let index = repository_index();
    let request = PackageInstallRequest {
        requested_ids: &["fcitx5-rime"],
        transaction_id: "install-rime",
    };
    let mut transport = FixtureTransport::with([vec![0_u8; 12], vec![0_u8; 12]]);
    let mut lock_fault = FailingExecutor::with_failure("lock_write_failed");
    let error = facade
        .install_or_update_from_repository(&index, request, &mut transport, &mut lock_fault)
        .expect_err("lock publication failure must fail the transaction");
    assert_eq!(error.code(), "lock_write_failed");
    assert!(lock_fault.rollback_called);

    let update = facade
        .activate_core_update(
            Path::new("missing.fcpkg"),
            "core-update",
            &fixture.install_root().join("security/trusted-keys.json"),
            "stable",
        )
        .expect_err("manual update owner must reject before archive access");
    assert_eq!(update.code(), "update_owner");

    let transaction = fixture
        .data_root()
        .join("packages/transactions/abandoned.state");
    std::fs::create_dir_all(transaction.parent().expect("transaction parent"))
        .expect("transaction parent should create");
    std::fs::write(
        &transaction,
        "format_version=1\ntransaction=abandoned\nstate=staged\n",
    )
    .expect("transaction journal should write");
    let report = facade
        .repair()
        .expect("repair should inspect abandoned transaction");
    assert!(report
        .actions()
        .contains(&PackageRepairAction::DiscardedIncompleteTransaction {
            transaction_id: "abandoned".to_owned(),
        }));
    assert!(
        !transaction.exists(),
        "repair must remove safe abandoned journal state"
    );
}

#[test]
fn successful_install_reports_exact_dependency_plan() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    let request = PackageInstallRequest {
        requested_ids: &["fcitx5-rime"],
        transaction_id: "install-rime",
    };
    let mut transport = FixtureTransport::with([vec![0_u8; 12], vec![0_u8; 12]]);
    let mut executor = RecordingExecutor::default();
    let result = facade
        .install_or_update_from_repository(
            &repository_index(),
            request,
            &mut transport,
            &mut executor,
        )
        .expect("valid exact plan should activate");
    assert_eq!(result.transaction_id(), "install-rime");
    assert_eq!(result.plan().entries().len(), 2);
    assert_eq!(executor.stages, 2);
    assert_eq!(executor.activations, 2);
}

#[test]
fn partial_multi_package_activation_restores_filesystem_and_lockfile() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    let package_root = fixture.data_root().join("packages");
    let old_payload = package_root.join("versions/old-addon/1.0.0/payload.txt");
    std::fs::create_dir_all(old_payload.parent().expect("old payload parent"))
        .expect("old payload parent should create");
    std::fs::write(&old_payload, b"known-good").expect("old payload should write");
    let old_lock = package_root.join("packages.lock");
    std::fs::write(&old_lock, b"known-good-lock").expect("old lock should write");

    let request = PackageInstallRequest {
        requested_ids: &["fcitx5-rime"],
        transaction_id: "transactional-install",
    };
    let archive = vec![0_u8; 12];
    let mut transport = FixtureTransport::with([archive.clone(), archive]);
    let mut executor = MutatingFailingExecutor::default();
    let error = facade
        .install_or_update_from_repository(
            &repository_index(),
            request,
            &mut transport,
            &mut executor,
        )
        .expect_err("a later package failure must fail the complete transaction");

    assert_eq!(error.code(), "activation_failed");
    assert!(executor.rollback_called);
    assert_eq!(
        std::fs::read(&old_payload).expect("old payload should be restored"),
        b"known-good"
    );
    assert_eq!(
        std::fs::read(&old_lock).expect("old lock should be restored"),
        b"known-good-lock"
    );
    assert!(
        !package_root.join("versions/partial-addon").exists(),
        "partial activation must not survive rollback"
    );
    assert!(!package_root
        .join("transactions/transactional-install.state")
        .exists());
    assert!(!package_root
        .join("transactions/transactional-install.backup")
        .exists());
    assert!(!package_root.join("staging/transactional-install").exists());
}

#[test]
fn archive_digest_mismatch_never_reaches_activation_and_activation_fault_rolls_back() {
    let fixture = FixtureRoot::new();
    let facade = fixture.facade();
    let index = repository_index();
    let request = PackageInstallRequest {
        requested_ids: &["fcitx5-rime"],
        transaction_id: "install-rime",
    };
    let mut mismatch_transport = FixtureTransport::with([b"wrong archive".to_vec()]);
    let mut executor = FailingExecutor::default();
    let mismatch = facade
        .install_or_update_from_repository(&index, request, &mut mismatch_transport, &mut executor)
        .expect_err("archive digest mismatch must fail");
    assert_eq!(mismatch.code(), "archive_digest_mismatch");
    assert!(!executor.rollback_called);

    let archive = vec![0_u8; 12];
    let mut matching_transport = FixtureTransport::with([archive, vec![0_u8; 12]]);
    let mut executor = FailingExecutor::default();
    let activation = facade
        .install_or_update_from_repository(&index, request, &mut matching_transport, &mut executor)
        .expect_err("activation fault must fail");
    assert_eq!(activation.code(), "activation_failed");
    assert!(executor.rollback_called);
}
