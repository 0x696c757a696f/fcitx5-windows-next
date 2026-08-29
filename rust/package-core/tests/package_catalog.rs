#![forbid(unsafe_code)]

use fcitx5_package_core::{
    parse_lockfile, parse_manifest, parse_repository_index_with_policy, read_package_catalog,
    read_package_detail, sha256_digest, BundledPackage, BundledPackageProbe, PackageCatalog,
    PackageCatalogEntry, PackageCatalogReadError, PackageCatalogReadOptions,
    PackageCatalogRepositoryError, PackageCatalogRepositoryRead, PackageCatalogRepositorySource,
    PackageCatalogSources, PackageConfigSurface, PackageId, RepositoryVerificationPolicy,
    SafeRelativePackagePath,
};
use std::path::{Path, PathBuf};

fn repository() -> fcitx5_package_core::RepositoryIndex {
    let rime_hash = "a".repeat(64);
    let data_hash = "b".repeat(64);
    let targets =
        format!("fcitx5-rime\t2.0.0\t2\tx64\t{rime_hash}\nrime-data\t1.0.0\t1\tany\t{data_hash}\n");
    let targets_sha256 = sha256_digest(targets.as_bytes());
    let index = format!(
        r#"{{"format_version":1,"repository_id":"fcitx5-windows-next","channel":"stable","mirror_id":"official","sequence":8,"generated_at":"2026-08-17T00:00:00Z","expires_at":"2026-08-21T00:00:00Z","key_id":"official-2026","targets":{{"count":2,"sha256":"{}"}},"packages":[{{"id":"fcitx5-rime","title":"Rime","summary":"Rime input method","version":"2.0.0","release_sequence":2,"type":"addon","architecture":"x64","download_url":"https://example.test/rime.fcpkg","sha256":"{rime_hash}","dependencies":[{{"id":"rime-data","version":"1.0.0"}}]}},{{"id":"rime-data","title":"Rime data","summary":"","version":"1.0.0","release_sequence":1,"type":"inputmethod-data","architecture":"any","download_url":"https://example.test/rime-data.fcpkg","sha256":"{data_hash}","dependencies":[]}}]}}"#,
        targets_sha256.as_str(),
    );
    let policy = RepositoryVerificationPolicy::new(
        "fcitx5-windows-next",
        "stable",
        "official",
        1_786_950_000,
    );
    parse_repository_index_with_policy(&index, &policy).expect("repository fixture should parse")
}

fn installed_manifest_json() -> &'static str {
    r#"{"format_version":1,"id":"fcitx5-rime","version":"1.0.0","type":"addon","architecture":"x64","min_os":"6.1-sp1","core_api":"1","addon_abi":"1","dependencies":[{"id":"rime-data","version":"1.0.0"}],"license":"MIT","source_commit":"0123456789abcdef","permissions":["native-code","input-data"],"files":[{"path":"lib/fcitx5/librime.dll","size":12,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},{"path":"share/fcitx5/addon/rime.conf","size":12,"sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},{"path":"share/rime-data/default.yaml","size":12,"sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}],"key_id":"release-2026"}"#
}

fn installed_manifest() -> fcitx5_package_core::Manifest {
    parse_manifest(installed_manifest_json()).expect("manifest fixture should parse")
}

fn installed_lock() -> Vec<fcitx5_package_core::LockEntry> {
    parse_lockfile(
        r#"{"format_version":1,"packages":[{"id":"fcitx5-rime","version":"1.0.0","manifest_sha256":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","state":"enabled"},{"id":"orphan","version":"0.1.0","manifest_sha256":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","state":"disabled"}]}"#,
    )
    .expect("lockfile fixture should parse")
}

fn bundled() -> Vec<BundledPackage> {
    vec![
        BundledPackage::new(
            PackageId::parse("fcitx5-lua").expect("bundled id should parse"),
            "Fcitx5 Lua".to_owned(),
            "2026.8.0".to_owned(),
        ),
        BundledPackage::new(
            PackageId::parse("fcitx5-rime").expect("bundled id should parse"),
            "Rime".to_owned(),
            "2026.8.0".to_owned(),
        ),
    ]
}

#[test]
fn catalog_merges_verified_repository_installed_bundle_and_detail_metadata() {
    let repository = repository();
    let installed = installed_lock();
    let manifest = installed_manifest();
    let bundles = bundled();
    let catalog = PackageCatalog::from_sources(PackageCatalogSources {
        repository: PackageCatalogRepositorySource::Verified(&repository),
        installed: &installed,
        installed_manifests: &[manifest],
        bundled: &bundles,
        architecture: "x64",
    });

    assert!(catalog.repository_available());
    assert_eq!(catalog.repository_error(), None);
    let metadata = catalog.repository().expect("metadata");
    assert_eq!(metadata.format_version(), 1);
    assert_eq!(metadata.sequence(), 8);
    let expected_targets = sha256_digest(
        format!(
            "fcitx5-rime\t2.0.0\t2\tx64\t{}\nrime-data\t1.0.0\t1\tany\t{}\n",
            "a".repeat(64),
            "b".repeat(64)
        )
        .as_bytes(),
    );
    assert_eq!(metadata.targets_sha256(), expected_targets.as_str());
    assert_eq!(
        catalog
            .packages()
            .iter()
            .map(|package| package.id())
            .collect::<Vec<_>>(),
        ["fcitx5-lua", "fcitx5-rime", "orphan", "rime-data"]
    );

    let rime = catalog.package("fcitx5-rime").expect("rime detail");
    assert_eq!(rime.title(), "Rime");
    assert_eq!(rime.available_version(), Some("2.0.0"));
    assert_eq!(rime.installed_version(), Some("1.0.0"));
    assert_eq!(rime.state(), Some("enabled"));
    assert!(rime.update_available());
    assert!(rime.bundled());
    assert_eq!(
        rime.dependencies()
            .iter()
            .map(|dependency| (dependency.id(), dependency.version()))
            .collect::<Vec<_>>(),
        [("rime-data", "1.0.0")]
    );
    assert_eq!(rime.permissions(), ["native-code", "input-data"]);
    assert_eq!(rime.manifest_sha256(), Some(&"d".repeat(64)[..]));
    assert_eq!(rime.source_commit(), Some("0123456789abcdef"));
    assert_eq!(
        rime.config_surfaces()
            .iter()
            .map(|surface| (surface.kind(), surface.owner(), surface.schema()))
            .collect::<Vec<_>>(),
        [
            ("fcitx-addon", "fcitx5-rime", "generic-fcitx-config-v1"),
            (
                "fcitx-addon-config",
                "fcitx5-rime",
                "generic-fcitx-config-v1"
            ),
            (
                "input-method-data",
                "fcitx5-rime",
                "generic-fcitx-config-v1"
            ),
            ("rime-data", "fcitx5-rime", "generic-fcitx-config-v1"),
        ]
    );
}

#[test]
fn catalog_preserves_unavailable_repository_error_and_local_records() {
    let installed = installed_lock();
    let bundles = bundled();
    let catalog = PackageCatalog::from_sources(PackageCatalogSources {
        repository: PackageCatalogRepositorySource::Unavailable(
            PackageCatalogRepositoryError::MissingKey,
        ),
        installed: &installed,
        installed_manifests: &[],
        bundled: &bundles,
        architecture: "x64",
    });

    assert!(!catalog.repository_available());
    assert_eq!(
        catalog.repository_error(),
        Some(PackageCatalogRepositoryError::MissingKey)
    );
    assert!(catalog.repository().is_none());
    assert_eq!(
        catalog
            .packages()
            .iter()
            .map(|package| (
                package.id(),
                package.package_type().as_str(),
                package.state()
            ))
            .collect::<Vec<_>>(),
        [
            ("fcitx5-lua", "addon", Some("bundled")),
            ("fcitx5-rime", "unknown", Some("enabled")),
            ("orphan", "unknown", Some("disabled")),
        ]
    );
    let rime = catalog.package("fcitx5-rime").expect("installed detail");
    assert_eq!(rime.title(), "fcitx5-rime");
    assert_eq!(rime.summary(), "");
    assert_eq!(rime.detail_title(), "fcitx5-rime");
    assert_eq!(
        rime.detail_summary(),
        "Bundled with Fcitx5 for Windows Next"
    );
    assert_eq!(rime.available_version(), None);
    assert!(!rime.update_available());
    assert_eq!(rime.permissions(), [] as [&str; 0]);
    assert_eq!(
        rime.config_surfaces()
            .iter()
            .map(PackageConfigSurface::kind)
            .collect::<Vec<_>>(),
        ["fcitx-addon"]
    );

    let lua = catalog.package("fcitx5-lua").expect("bundled detail");
    assert_eq!(lua.title(), "Fcitx5 Lua");
    assert_eq!(lua.detail_title(), "fcitx5-lua");
}

struct FixtureRoot(PathBuf);

impl FixtureRoot {
    fn new() -> Self {
        let unique = format!(
            "fcitx5-package-catalog-{}-{}",
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

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn filesystem_reader_owns_repository_lock_manifest_bundle_and_detail_errors() {
    let fixture = FixtureRoot::new();
    let install_root = fixture.path().join("install");
    let data_root = fixture.path().join("data");
    let package_root = data_root.join("packages");
    let manifest_path = package_root
        .join("manifests/fcitx5-rime")
        .join("1.0.0.json");
    std::fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
        .expect("manifest directory should create");
    std::fs::write(&manifest_path, installed_manifest_json())
        .expect("manifest fixture should write");
    let manifest_sha256 = sha256_digest(installed_manifest_json().as_bytes());
    std::fs::write(
        package_root.join("packages.lock"),
        format!(
            r#"{{"format_version":1,"packages":[{{"id":"fcitx5-rime","version":"1.0.0","manifest_sha256":"{}","state":"enabled"}}]}}"#,
            manifest_sha256.as_str()
        ),
    )
    .expect("lock fixture should write");
    let bundled_path = install_root.join("lib/fcitx5/libluaaddonloader.dll");
    std::fs::create_dir_all(bundled_path.parent().expect("bundled parent"))
        .expect("bundled directory should create");
    std::fs::write(&bundled_path, b"fixture").expect("bundled probe should write");

    let repository = repository();
    let probes = [BundledPackageProbe::new(
        PackageId::parse("fcitx5-lua").expect("probe id should parse"),
        "Fcitx5 Lua".to_owned(),
        SafeRelativePackagePath::parse("lib/fcitx5/libluaaddonloader.dll")
            .expect("probe path should parse"),
    )];
    let options = PackageCatalogReadOptions {
        expected_channel: "stable",
        architecture: "x64",
        release_version: "2026.8.0",
        bundled: &probes,
        repository: PackageCatalogRepositoryRead::Verified(&repository),
    };
    let catalog = read_package_catalog(&install_root, &data_root, options)
        .expect("filesystem catalog should read");
    assert!(catalog.repository_available());
    assert_eq!(
        catalog
            .packages()
            .iter()
            .map(PackageCatalogEntry::id)
            .collect::<Vec<_>>(),
        ["fcitx5-lua", "fcitx5-rime", "rime-data"]
    );
    let rime = catalog.package("fcitx5-rime").expect("installed package");
    assert_eq!(rime.manifest_sha256(), Some(manifest_sha256.as_str()));
    assert_eq!(rime.permissions(), ["native-code", "input-data"]);
    let lua = catalog.package("fcitx5-lua").expect("bundled package");
    assert_eq!(lua.title(), "Fcitx5 Lua");
    assert_eq!(lua.detail_title(), "fcitx5-lua");

    let detail = read_package_detail(&install_root, &data_root, "fcitx5-rime", options)
        .expect("known detail should read");
    assert_eq!(detail.source_commit(), Some("0123456789abcdef"));
    assert_eq!(
        read_package_detail(&install_root, &data_root, "Bad ID", options)
            .expect_err("invalid id should fail"),
        PackageCatalogReadError::InvalidPackage
    );
    assert_eq!(
        read_package_detail(&install_root, &data_root, "unknown", options)
            .expect_err("unknown package should fail"),
        PackageCatalogReadError::PackageNotFound
    );

    let unavailable = read_package_catalog(
        &install_root,
        &data_root,
        PackageCatalogReadOptions {
            repository: PackageCatalogRepositoryRead::Cached,
            ..options
        },
    )
    .expect("missing repository remains a usable local catalog");
    assert_eq!(
        unavailable.repository_error(),
        Some(PackageCatalogRepositoryError::RepositoryUnavailable)
    );
    assert!(unavailable.package("fcitx5-rime").is_some());
    assert!(unavailable.package("fcitx5-lua").is_some());
}
