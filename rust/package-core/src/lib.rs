#![forbid(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeSet;
use std::fmt;

const MANIFEST_FORMAT_VERSION_V1: u64 = 1;
const MANIFEST_FORMAT_VERSION_V2: u64 = 2;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_PACKAGE_ID_BYTES: usize = 64;
const MAX_VERSION_BYTES: usize = 64;
const MAX_METADATA_BYTES: usize = 256;
const MAX_PACKAGE_PATH_BYTES: usize = 512;
const MAX_DEPENDENCY_COUNT: usize = 256;
const MAX_PERMISSION_COUNT: usize = 32;
const MAX_FILE_COUNT: usize = 4096;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PAYLOAD_BYTES: u64 = 512 * 1024 * 1024;
const SUPPORTED_CORE_API: &str = "1";
const SUPPORTED_ADDON_ABI: &str = "1";
const SIGNATURE_ENVELOPE_CANONICALIZATION: &str = "fcitx5-windows-next-json-v1";
const MLDSA65_PUBLIC_KEY_BYTES: usize = 1952;
const SLHDSA_SHA2_128S_PUBLIC_KEY_BYTES: usize = 32;
const RSA_PUBLIC_MAGIC: u32 = 0x3141_5352;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PackageId(String);

impl PackageId {
    pub fn parse(value: &str) -> Result<Self, PackageIdError> {
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_PACKAGE_ID_BYTES
            || !bytes[0].is_ascii_lowercase()
            || !bytes.iter().all(|&byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'-' | b'_')
            })
        {
            return Err(PackageIdError);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageIdError;

impl fmt::Display for PackageIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid package id")
    }
}

impl std::error::Error for PackageIdError {}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SafeRelativePackagePath(String);

impl SafeRelativePackagePath {
    pub fn parse(value: &str) -> Result<Self, PackagePathError> {
        validate_package_path(value)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackagePathError {
    Empty,
    TooLong,
    AbsoluteOrRooted,
    BackslashSeparator,
    AlternateDataStream,
    EmptyComponent,
    DotComponent,
    ParentComponent,
    TrailingDotOrSpace,
    ControlCharacter,
    DosDeviceComponent,
}

impl fmt::Display for PackagePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "empty path",
            Self::TooLong => "path exceeds package path budget",
            Self::AbsoluteOrRooted => "absolute or rooted path",
            Self::BackslashSeparator => "backslash separator",
            Self::AlternateDataStream => "alternate data stream marker",
            Self::EmptyComponent => "empty path component",
            Self::DotComponent => "dot path component",
            Self::ParentComponent => "parent path component",
            Self::TrailingDotOrSpace => "component ends with dot or space",
            Self::ControlCharacter => "control character in path component",
            Self::DosDeviceComponent => "DOS device path component",
        })
    }
}

impl std::error::Error for PackagePathError {}

pub fn is_safe_relative_package_path(value: &str) -> bool {
    validate_package_path(value).is_ok()
}

fn validate_package_path(value: &str) -> Result<(), PackagePathError> {
    if value.is_empty() {
        return Err(PackagePathError::Empty);
    }
    if value.len() > MAX_PACKAGE_PATH_BYTES {
        return Err(PackagePathError::TooLong);
    }
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.ends_with('/')
        || value.ends_with('\\')
    {
        return Err(PackagePathError::AbsoluteOrRooted);
    }
    if value.contains('\\') {
        return Err(PackagePathError::BackslashSeparator);
    }
    if value.contains(':') {
        return Err(PackagePathError::AlternateDataStream);
    }
    if value.as_bytes().contains(&0) {
        return Err(PackagePathError::ControlCharacter);
    }

    for component in value.split('/') {
        validate_component(component)?;
    }
    Ok(())
}

fn validate_component(component: &str) -> Result<(), PackagePathError> {
    if component.is_empty() {
        return Err(PackagePathError::EmptyComponent);
    }
    if component == "." {
        return Err(PackagePathError::DotComponent);
    }
    if component == ".." {
        return Err(PackagePathError::ParentComponent);
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return Err(PackagePathError::TrailingDotOrSpace);
    }
    if component.bytes().any(|byte| byte < 0x20) {
        return Err(PackagePathError::ControlCharacter);
    }

    let stem = component
        .split_once('.')
        .map_or(component, |(stem, _)| stem);
    let lowered = stem.to_ascii_lowercase();
    if matches!(lowered.as_str(), "con" | "prn" | "aux" | "nul")
        || (lowered.len() == 4
            && ((lowered.starts_with("com") && matches!(lowered.as_bytes()[3], b'1'..=b'9'))
                || (lowered.starts_with("lpt") && matches!(lowered.as_bytes()[3], b'1'..=b'9'))))
    {
        return Err(PackagePathError::DosDeviceComponent);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HexDigest32(String);

impl HexDigest32 {
    pub fn parse(value: &str) -> Result<Self, HexDigestError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HexDigestError);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HexDigestError;

impl fmt::Display for HexDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid 32-byte hex digest")
    }
}

impl std::error::Error for HexDigestError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadHashes {
    blake3: Option<HexDigest32>,
    sha256: Option<HexDigest32>,
}

impl PayloadHashes {
    pub fn v1_sha256(sha256: HexDigest32) -> Self {
        Self {
            blake3: None,
            sha256: Some(sha256),
        }
    }

    pub fn v2_blake3(blake3: HexDigest32, sha256: Option<HexDigest32>) -> Self {
        Self {
            blake3: Some(blake3),
            sha256,
        }
    }

    pub fn blake3(&self) -> Option<&HexDigest32> {
        self.blake3.as_ref()
    }

    pub fn sha256(&self) -> Option<&HexDigest32> {
        self.sha256.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArtifact {
    package_id: PackageId,
    path: SafeRelativePackagePath,
    size: u64,
    hashes: PayloadHashes,
}

impl VerifiedArtifact {
    pub fn new(
        package_id: PackageId,
        path: SafeRelativePackagePath,
        size: u64,
        hashes: PayloadHashes,
    ) -> Self {
        Self {
            package_id,
            path,
            size,
            hashes,
        }
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn path(&self) -> &SafeRelativePackagePath {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn hashes(&self) -> &PayloadHashes {
        &self.hashes
    }
}

pub fn has_windows_ordinal_case_collision<'a>(
    paths: impl IntoIterator<Item = &'a SafeRelativePackagePath>,
) -> bool {
    let mut seen = BTreeSet::new();
    for path in paths {
        let key = path.as_str().to_lowercase();
        if !seen.insert(key) {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageType {
    Core,
    Addon,
    InputMethodData,
    Theme,
    Translation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    id: PackageId,
    version: String,
}

impl Dependency {
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    format_version: u64,
    id: PackageId,
    version: String,
    package_type: PackageType,
    architecture: String,
    min_os: String,
    core_api: String,
    addon_abi: String,
    dependencies: Vec<Dependency>,
    license: String,
    source_commit: String,
    permissions: Vec<String>,
    files: Vec<VerifiedArtifact>,
    key_id: PackageId,
}

impl Manifest {
    pub fn format_version(&self) -> u64 {
        self.format_version
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn package_type(&self) -> &PackageType {
        &self.package_type
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    pub fn min_os(&self) -> &str {
        &self.min_os
    }

    pub fn core_api(&self) -> &str {
        &self.core_api
    }

    pub fn addon_abi(&self) -> &str {
        &self.addon_abi
    }

    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    pub fn license(&self) -> &str {
        &self.license
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    pub fn files(&self) -> &[VerifiedArtifact] {
        &self.files
    }

    pub fn key_id(&self) -> &PackageId {
        &self.key_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestError {
    code: &'static str,
    message: String,
}

impl ManifestError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ManifestError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityError {
    code: &'static str,
    message: String,
}

impl CompatibilityError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompatibilityError {}

pub fn parse_manifest(bytes: &str) -> Result<Manifest, ManifestError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(manifest_error(
            "invalid_manifest",
            "manifest size is outside the accepted range",
        ));
    }

    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|message| manifest_error("invalid_manifest", message))?;
    let object = document
        .as_object()
        .ok_or_else(|| manifest_error("invalid_manifest", "expected a JSON object"))?;
    require_object_keys(
        object,
        &[
            "format_version",
            "id",
            "version",
            "type",
            "architecture",
            "min_os",
            "core_api",
            "addon_abi",
            "dependencies",
            "license",
            "source_commit",
            "permissions",
            "key_id",
        ],
        &["files", "payload"],
    )?;

    let format_version = require_unsigned(object, "format_version", "unsupported_manifest")?;
    if format_version != MANIFEST_FORMAT_VERSION_V1 && format_version != MANIFEST_FORMAT_VERSION_V2
    {
        return Err(manifest_error(
            "unsupported_manifest",
            "format_version is unsupported",
        ));
    }
    if (format_version == MANIFEST_FORMAT_VERSION_V1
        && (!object_contains(object, "files") || object_contains(object, "payload")))
        || (format_version == MANIFEST_FORMAT_VERSION_V2
            && (!object_contains(object, "payload") || object_contains(object, "files")))
    {
        return Err(manifest_error(
            "invalid_manifest",
            "manifest payload schema does not match format version",
        ));
    }

    let id = PackageId::parse(&require_string(object, "id", MAX_PACKAGE_ID_BYTES, false)?)
        .map_err(|_| manifest_error("invalid_manifest", "package identity is invalid"))?;
    let version = require_ascii_token_string(object, "version", MAX_VERSION_BYTES, ".+-_")?;
    let package_type = parse_package_type(&require_string(object, "type", 32, false)?)?;
    let architecture = require_string(object, "architecture", 8, false)?;
    if !matches!(architecture.as_str(), "any" | "x86" | "x64") {
        return Err(manifest_error(
            "invalid_manifest",
            "package architecture is invalid",
        ));
    }
    let min_os = require_string(object, "min_os", 32, false)?;
    let core_api = require_string(object, "core_api", MAX_VERSION_BYTES, false)?;
    let addon_abi = require_string(object, "addon_abi", MAX_VERSION_BYTES, true)?;
    let license = require_string(object, "license", MAX_METADATA_BYTES, false)?;
    let source_commit = require_string(object, "source_commit", 128, false)?;
    let key_id = PackageId::parse(&require_string(
        object,
        "key_id",
        MAX_PACKAGE_ID_BYTES,
        false,
    )?)
    .map_err(|_| manifest_error("invalid_manifest", "package key id is invalid"))?;

    let dependencies = parse_dependencies(require_array(object, "dependencies")?)?;
    let permissions = parse_permissions(require_array(object, "permissions")?)?;
    let files = parse_manifest_files(format_version, &id, object)?;

    Ok(Manifest {
        format_version,
        id,
        version,
        package_type,
        architecture,
        min_os,
        core_api,
        addon_abi,
        dependencies,
        license,
        source_commit,
        permissions,
        files,
        key_id,
    })
}

pub fn validate_manifest_compatibility(
    manifest: &Manifest,
    architecture: &str,
) -> Result<(), CompatibilityError> {
    if !matches!(architecture, "x64" | "x86") {
        return Err(compatibility_error("runtime architecture is invalid"));
    }
    if manifest.architecture() != "any" && manifest.architecture() != architecture {
        return Err(compatibility_error(
            "package architecture does not match this runtime",
        ));
    }
    if manifest.core_api() != SUPPORTED_CORE_API {
        return Err(compatibility_error(
            "package requires an unsupported Core API",
        ));
    }
    if manifest.package_type() == &PackageType::Addon && manifest.addon_abi() != SUPPORTED_ADDON_ABI
    {
        return Err(compatibility_error("addon ABI does not match this engine"));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustAlgorithm {
    Rsa2048Sha256,
    Mldsa65,
    SlhdsaSha2_128s,
}

impl TrustAlgorithm {
    fn parse_for_keyring(value: &str, format_version: u64) -> Result<Self, KeyringError> {
        match value {
            "rsa-2048-sha256" => Ok(Self::Rsa2048Sha256),
            "mldsa65" if format_version == 2 => Ok(Self::Mldsa65),
            "slhdsa-sha2-128s" if format_version == 2 => Ok(Self::SlhdsaSha2_128s),
            _ => Err(keyring_error("trusted key algorithm is unsupported")),
        }
    }

    fn parse_for_signature(value: &str) -> Result<Self, SignatureEnvelopeError> {
        match value {
            "mldsa65" => Ok(Self::Mldsa65),
            "slhdsa-sha2-128s" => Ok(Self::SlhdsaSha2_128s),
            _ => Err(signature_error(
                "signature envelope requires an unsupported algorithm",
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rsa2048Sha256 => "rsa-2048-sha256",
            Self::Mldsa65 => "mldsa65",
            Self::SlhdsaSha2_128s => "slhdsa-sha2-128s",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedKey {
    id: PackageId,
    algorithm: TrustAlgorithm,
    public_key: Vec<u8>,
    revoked: bool,
}

impl TrustedKey {
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn algorithm(&self) -> &TrustAlgorithm {
        &self.algorithm
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn revoked(&self) -> bool {
        self.revoked
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyringError {
    code: &'static str,
    message: String,
}

impl KeyringError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for KeyringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for KeyringError {}

pub fn parse_trusted_keys(bytes: &str) -> Result<Vec<TrustedKey>, KeyringError> {
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| keyring_error("trusted key file is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;
    if !object_contains(object, "format_version") {
        return Err(keyring_error("trusted key file schema is invalid"));
    }
    let format_version = object_get(object, "format_version")
        .and_then(JsonValue::as_number)
        .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;

    match format_version {
        1 => require_object_keys_with_code(
            object,
            &["format_version", "keys"],
            &[],
            "invalid_keyring",
        )?,
        2 => {
            require_object_keys_with_code(
                object,
                &["format_version", "policy", "keys"],
                &[],
                "invalid_keyring",
            )?;
            validate_keyring_policy(require_object_for_code(
                object,
                "policy",
                "invalid_keyring",
            )?)?;
        }
        _ => return Err(keyring_error("trusted key format version is unsupported")),
    }

    let keys = object_get(object, "keys")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;
    if keys.len() > 64 {
        return Err(keyring_error("trusted key file schema is invalid"));
    }

    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in keys {
        let key = value
            .as_object()
            .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;
        if format_version == 1 {
            require_object_keys_with_code(
                key,
                &["key_id", "algorithm", "status", "public_key_base64"],
                &[],
                "invalid_keyring",
            )?;
        } else {
            require_object_keys_with_code(
                key,
                &[
                    "key_id",
                    "algorithm",
                    "status",
                    "public_key_base64",
                    "scope",
                    "channels",
                ],
                &[],
                "invalid_keyring",
            )?;
            validate_non_empty_bounded_array(key, "scope", 8)?;
            validate_non_empty_bounded_array(key, "channels", 16)?;
        }

        let id = PackageId::parse(&require_string_for_code(
            key,
            "key_id",
            MAX_PACKAGE_ID_BYTES,
            false,
            "invalid_keyring",
        )?)
        .map_err(|_| keyring_error("trusted key record is invalid"))?;
        let algorithm = TrustAlgorithm::parse_for_keyring(
            &require_string_for_code(key, "algorithm", 32, false, "invalid_keyring")?,
            format_version,
        )?;
        let status = require_string_for_code(key, "status", 16, false, "invalid_keyring")?;
        let public_key = decode_base64(&require_string_for_code(
            key,
            "public_key_base64",
            16384,
            false,
            "invalid_keyring",
        )?)
        .map_err(|_| keyring_error("base64 decoding failed"))?;

        if !matches!(status.as_str(), "trusted" | "revoked") || !ids.insert(id.as_str().to_owned())
        {
            return Err(keyring_error("trusted key record is invalid"));
        }
        validate_public_key(&algorithm, &public_key)?;
        result.push(TrustedKey {
            id,
            algorithm,
            public_key,
            revoked: status == "revoked",
        });
    }

    Ok(result)
}

fn validate_keyring_policy(policy: &[(String, JsonValue)]) -> Result<(), KeyringError> {
    require_object_keys_with_code(
        policy,
        &[
            "official_required_signatures",
            "compatibility_hashes",
            "default_payload_hash",
        ],
        &[],
        "invalid_keyring",
    )?;
    let official = object_get(policy, "official_required_signatures")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key policy is invalid"))?;
    let compatibility = object_get(policy, "compatibility_hashes")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key policy is invalid"))?;
    if official.len() > 8
        || compatibility.len() > 8
        || require_string_for_code(policy, "default_payload_hash", 32, false, "invalid_keyring")?
            != "blake3"
    {
        return Err(keyring_error("trusted key policy is invalid"));
    }
    for algorithm in official {
        let Some(value) = algorithm.as_string() else {
            return Err(keyring_error("trusted key policy algorithm is invalid"));
        };
        if !matches!(value, "mldsa65" | "slhdsa-sha2-128s") {
            return Err(keyring_error(
                "trusted key policy requires unsupported algorithm",
            ));
        }
    }
    Ok(())
}

fn validate_non_empty_bounded_array(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
) -> Result<(), KeyringError> {
    let array = object_get(object, key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key scope/channel policy is invalid"))?;
    if array.is_empty() || array.len() > maximum {
        return Err(keyring_error("trusted key scope/channel policy is invalid"));
    }
    Ok(())
}

fn validate_public_key(algorithm: &TrustAlgorithm, public_key: &[u8]) -> Result<(), KeyringError> {
    match algorithm {
        TrustAlgorithm::Rsa2048Sha256 => {
            if public_key.len() < 8 {
                return Err(keyring_error("RSA public key blob is truncated"));
            }
            let magic =
                u32::from_le_bytes([public_key[0], public_key[1], public_key[2], public_key[3]]);
            let bit_length =
                u32::from_le_bytes([public_key[4], public_key[5], public_key[6], public_key[7]]);
            if magic != RSA_PUBLIC_MAGIC || !(2048..=4096).contains(&bit_length) {
                return Err(keyring_error(
                    "RSA public key strength or representation is invalid",
                ));
            }
            Ok(())
        }
        TrustAlgorithm::Mldsa65 if public_key.len() == MLDSA65_PUBLIC_KEY_BYTES => Ok(()),
        TrustAlgorithm::Mldsa65 => Err(keyring_error("ML-DSA-65 public key length is invalid")),
        TrustAlgorithm::SlhdsaSha2_128s
            if public_key.len() == SLHDSA_SHA2_128S_PUBLIC_KEY_BYTES =>
        {
            Ok(())
        }
        TrustAlgorithm::SlhdsaSha2_128s => {
            Err(keyring_error("SLH-DSA public key length is invalid"))
        }
    }
}

fn keyring_error(message: impl Into<String>) -> KeyringError {
    KeyringError {
        code: "invalid_keyring",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignedObject {
    RepositoryIndex,
    PackageManifest,
}

impl SignedObject {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "repository-index" => Some(Self::RepositoryIndex),
            "package-manifest" => Some(Self::PackageManifest),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RepositoryIndex => "repository-index",
            Self::PackageManifest => "package-manifest",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureEnvelopeEntry {
    key_id: PackageId,
    algorithm: TrustAlgorithm,
    signature: Vec<u8>,
}

impl SignatureEnvelopeEntry {
    pub fn key_id(&self) -> &PackageId {
        &self.key_id
    }

    pub fn algorithm(&self) -> &TrustAlgorithm {
        &self.algorithm
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureEnvelope {
    format_version: u64,
    signed_object: SignedObject,
    canonicalization: String,
    signatures: Vec<SignatureEnvelopeEntry>,
}

impl SignatureEnvelope {
    pub fn format_version(&self) -> u64 {
        self.format_version
    }

    pub fn signed_object(&self) -> &SignedObject {
        &self.signed_object
    }

    pub fn canonicalization(&self) -> &str {
        &self.canonicalization
    }

    pub fn signatures(&self) -> &[SignatureEnvelopeEntry] {
        &self.signatures
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureEnvelopeError {
    code: &'static str,
    message: String,
}

impl SignatureEnvelopeError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for SignatureEnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SignatureEnvelopeError {}

pub fn parse_signature_envelope(
    bytes: &str,
    expected_object: SignedObject,
) -> Result<SignatureEnvelope, SignatureEnvelopeError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(signature_error("signature envelope identity is invalid"));
    }
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| signature_error("signature envelope is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| signature_error("signature envelope entry must be a JSON object"))?;
    require_object_keys_with_code(
        object,
        &[
            "format_version",
            "signed_object",
            "canonicalization",
            "signatures",
        ],
        &[],
        "invalid_signature",
    )
    .map_err(|error| signature_error(error.message))?;

    let format_version = object_get(object, "format_version")
        .and_then(JsonValue::as_number)
        .ok_or_else(|| signature_error("signature envelope format version is unsupported"))?;
    if format_version != 2 {
        return Err(signature_error(
            "signature envelope format version is unsupported",
        ));
    }

    let signed_object_name =
        require_signature_string(object, "signed_object", 64).map_err(signature_error)?;
    let signed_object = SignedObject::parse(&signed_object_name)
        .ok_or_else(|| signature_error("signature envelope object binding is invalid"))?;
    let canonicalization =
        require_signature_string(object, "canonicalization", 64).map_err(signature_error)?;
    if signed_object != expected_object || canonicalization != SIGNATURE_ENVELOPE_CANONICALIZATION {
        return Err(signature_error(
            "signature envelope object binding is invalid",
        ));
    }

    let signatures = object_get(object, "signatures")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| signature_error("signature envelope signatures array is invalid"))?;
    if signatures.is_empty() || signatures.len() > 16 {
        return Err(signature_error(
            "signature envelope signatures array is invalid",
        ));
    }

    let mut key_ids = BTreeSet::new();
    let mut has_required_mldsa65 = false;
    let mut parsed = Vec::new();
    for value in signatures {
        let entry = value
            .as_object()
            .ok_or_else(|| signature_error("signature envelope entry must be a JSON object"))?;
        require_object_keys_with_code(
            entry,
            &["key_id", "algorithm", "signature_base64"],
            &[],
            "invalid_signature",
        )
        .map_err(|error| signature_error(error.message))?;
        let key_id = PackageId::parse(
            &require_signature_string(entry, "key_id", MAX_PACKAGE_ID_BYTES)
                .map_err(signature_error)?,
        )
        .map_err(|_| signature_error("signature envelope key id is invalid or duplicated"))?;
        if !key_ids.insert(key_id.as_str().to_owned()) {
            return Err(signature_error(
                "signature envelope key id is invalid or duplicated",
            ));
        }
        let algorithm = TrustAlgorithm::parse_for_signature(
            &require_signature_string(entry, "algorithm", 32).map_err(signature_error)?,
        )?;
        if algorithm == TrustAlgorithm::Mldsa65 {
            has_required_mldsa65 = true;
        }
        let signature = decode_base64(
            &require_signature_string(entry, "signature_base64", 16384).map_err(signature_error)?,
        )
        .map_err(|_| signature_error("base64 decoding failed"))?;
        parsed.push(SignatureEnvelopeEntry {
            key_id,
            algorithm,
            signature,
        });
    }
    if !has_required_mldsa65 {
        return Err(signature_error(
            "signature envelope is missing required ML-DSA-65 signature",
        ));
    }

    Ok(SignatureEnvelope {
        format_version,
        signed_object,
        canonicalization,
        signatures: parsed,
    })
}

fn require_signature_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
) -> Result<String, String> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| format!("{key} must be a string"))?;
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(format!("{key} has an invalid length"));
    }
    Ok(value.to_owned())
}

fn signature_error(message: impl Into<String>) -> SignatureEnvelopeError {
    SignatureEnvelopeError {
        code: "invalid_signature",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionError {
    code: &'static str,
    message: String,
}

impl ResolutionError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResolutionError {}

pub fn resolve_exact_dependencies(
    available: &[Manifest],
    requested_ids: &[&str],
) -> Result<Vec<String>, ResolutionError> {
    let mut packages = Vec::<(&str, &Manifest)>::new();
    for package in available {
        if packages.iter().any(|(id, _)| *id == package.id().as_str()) {
            return Err(resolution_error("repository contains duplicate package id"));
        }
        packages.push((package.id().as_str(), package));
    }

    let mut visits = Vec::<(&str, Visit)>::new();
    let mut result = Vec::new();
    for id in requested_ids {
        visit_dependency(id, &packages, &mut visits, &mut result)?;
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Visit {
    Visiting,
    Complete,
}

fn visit_dependency<'a>(
    id: &'a str,
    packages: &[(&'a str, &'a Manifest)],
    visits: &mut Vec<(&'a str, Visit)>,
    result: &mut Vec<String>,
) -> Result<(), ResolutionError> {
    if let Some((_, state)) = visits.iter().find(|(visited_id, _)| *visited_id == id) {
        return match state {
            Visit::Visiting => Err(resolution_error("dependency cycle detected")),
            Visit::Complete => Ok(()),
        };
    }
    let manifest = packages
        .iter()
        .find_map(|(package_id, package)| (*package_id == id).then_some(*package))
        .ok_or_else(|| resolution_error(format!("required package is unavailable: {id}")))?;

    visits.push((id, Visit::Visiting));
    for dependency in manifest.dependencies() {
        let target = packages
            .iter()
            .find_map(|(package_id, package)| {
                (*package_id == dependency.id().as_str()).then_some(*package)
            })
            .ok_or_else(|| resolution_error("exact dependency version is unavailable"))?;
        if target.version() != dependency.version() {
            return Err(resolution_error("exact dependency version is unavailable"));
        }
        visit_dependency(dependency.id().as_str(), packages, visits, result)?;
    }
    if let Some((_, state)) = visits.iter_mut().find(|(visited_id, _)| *visited_id == id) {
        *state = Visit::Complete;
    }
    result.push(id.to_owned());
    Ok(())
}

fn resolution_error(message: impl Into<String>) -> ResolutionError {
    ResolutionError {
        code: "resolution_failed",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadEntry {
    path: SafeRelativePackagePath,
    size: u64,
}

impl PayloadEntry {
    pub fn new(path: SafeRelativePackagePath, size: u64) -> Self {
        Self { path, size }
    }

    pub fn path(&self) -> &SafeRelativePackagePath {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadError {
    code: &'static str,
    message: String,
}

impl PayloadError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PayloadError {}

pub fn verify_payload_inventory(
    manifest: &Manifest,
    observed: &[PayloadEntry],
) -> Result<(), PayloadError> {
    let mut declared = Vec::<(&str, u64, bool)>::new();
    for file in manifest.files() {
        declared.push((file.path().as_str(), file.size(), false));
    }
    let mut observed_exact = BTreeSet::new();
    let mut observed_windows = BTreeSet::new();
    for entry in observed {
        if !observed_exact.insert(entry.path().as_str().to_owned())
            || !observed_windows.insert(entry.path().as_str().to_lowercase())
        {
            return Err(payload_error(
                "payload contains duplicate or colliding files",
            ));
        }
        let Some((_, declared_size, seen)) = declared
            .iter_mut()
            .find(|(path, _, _)| *path == entry.path().as_str())
        else {
            return Err(payload_error("payload contains an undeclared file"));
        };
        if *declared_size != entry.size() {
            return Err(payload_error("payload file does not match manifest"));
        }
        *seen = true;
    }
    if declared.iter().any(|(_, _, seen)| !*seen) {
        return Err(payload_error("payload file does not match manifest"));
    }
    Ok(())
}

fn payload_error(message: impl Into<String>) -> PayloadError {
    PayloadError {
        code: "payload_mismatch",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadDigestEntry {
    path: SafeRelativePackagePath,
    size: u64,
    blake3: Option<HexDigest32>,
    sha256: Option<HexDigest32>,
}

impl PayloadDigestEntry {
    pub fn new(
        path: SafeRelativePackagePath,
        size: u64,
        blake3: Option<HexDigest32>,
        sha256: Option<HexDigest32>,
    ) -> Self {
        Self {
            path,
            size,
            blake3,
            sha256,
        }
    }
}

pub fn verify_payload_digests(
    manifest: &Manifest,
    observed: &[PayloadDigestEntry],
) -> Result<(), PayloadError> {
    let inventory: Vec<_> = observed
        .iter()
        .map(|entry| PayloadEntry::new(entry.path.clone(), entry.size))
        .collect();
    verify_payload_inventory(manifest, &inventory)?;

    for file in manifest.files() {
        let observed_file = observed
            .iter()
            .find(|entry| entry.path.as_str() == file.path().as_str())
            .ok_or_else(|| payload_error("payload file does not match manifest"))?;
        if manifest.format_version() == MANIFEST_FORMAT_VERSION_V1 {
            if file.hashes().sha256() != observed_file.sha256.as_ref() {
                return Err(payload_error("payload file does not match manifest"));
            }
        } else if manifest.format_version() == MANIFEST_FORMAT_VERSION_V2 {
            if file.hashes().blake3() != observed_file.blake3.as_ref()
                || (file.hashes().sha256().is_some()
                    && file.hashes().sha256() != observed_file.sha256.as_ref())
            {
                return Err(payload_error("payload file does not match manifest"));
            }
        } else {
            return Err(payload_error(
                "payload verifier does not support manifest version",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageLifecycleState {
    Installed,
    Enabled,
    Disabled,
    PendingUpdate,
    PendingRemove,
    Broken,
    Quarantined,
}

impl PackageLifecycleState {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "installed" => Some(Self::Installed),
            "enabled" => Some(Self::Enabled),
            "disabled" => Some(Self::Disabled),
            "pending_update" => Some(Self::PendingUpdate),
            "pending_remove" => Some(Self::PendingRemove),
            "broken" => Some(Self::Broken),
            "quarantined" => Some(Self::Quarantined),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::PendingUpdate => "pending_update",
            Self::PendingRemove => "pending_remove",
            Self::Broken => "broken",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEntry {
    id: PackageId,
    version: String,
    manifest_sha256: HexDigest32,
    state: PackageLifecycleState,
}

impl LockEntry {
    pub fn new(
        id: PackageId,
        version: String,
        manifest_sha256: HexDigest32,
        state: PackageLifecycleState,
    ) -> Self {
        Self {
            id,
            version,
            manifest_sha256,
            state,
        }
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn manifest_sha256(&self) -> &HexDigest32 {
        &self.manifest_sha256
    }

    pub fn state(&self) -> &PackageLifecycleState {
        &self.state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockfileError {
    code: &'static str,
    message: String,
}

impl LockfileError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for LockfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LockfileError {}

pub fn parse_lockfile(bytes: &str) -> Result<Vec<LockEntry>, LockfileError> {
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| lockfile_error("packages.lock is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| lockfile_error("packages.lock schema is invalid"))?;
    require_object_keys_for_lockfile(object, &["format_version", "packages"], &[])?;
    let format_version = object_get(object, "format_version")
        .and_then(JsonValue::as_number)
        .ok_or_else(|| lockfile_error("packages.lock schema is invalid"))?;
    let packages = object_get(object, "packages")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| lockfile_error("packages.lock schema is invalid"))?;
    if format_version != 1 || packages.len() > MAX_FILE_COUNT {
        return Err(lockfile_error("packages.lock schema is invalid"));
    }

    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in packages {
        let entry = value
            .as_object()
            .ok_or_else(|| lockfile_error("packages.lock entry is invalid"))?;
        require_object_keys_for_lockfile(
            entry,
            &["id", "version", "manifest_sha256", "state"],
            &[],
        )?;
        let id = PackageId::parse(&require_lockfile_string(
            entry,
            "id",
            MAX_PACKAGE_ID_BYTES,
            false,
        )?)
        .map_err(|_| lockfile_error("packages.lock entry is invalid"))?;
        if !ids.insert(id.as_str().to_owned()) {
            return Err(lockfile_error("packages.lock entry is invalid"));
        }
        let version = require_lockfile_string(entry, "version", MAX_VERSION_BYTES, false)?;
        let manifest_sha256 = HexDigest32::parse(&require_lockfile_string(
            entry,
            "manifest_sha256",
            64,
            false,
        )?)
        .map_err(|_| lockfile_error("packages.lock entry is invalid"))?;
        let state =
            PackageLifecycleState::parse(&require_lockfile_string(entry, "state", 32, false)?)
                .ok_or_else(|| lockfile_error("packages.lock entry is invalid"))?;
        if !is_ascii_token(&version, ".+-_") {
            return Err(lockfile_error("packages.lock entry is invalid"));
        }
        result.push(LockEntry {
            id,
            version,
            manifest_sha256,
            state,
        });
    }
    Ok(result)
}

fn require_object_keys_for_lockfile(
    object: &[(String, JsonValue)],
    required: &[&str],
    optional: &[&str],
) -> Result<(), LockfileError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(lockfile_error(format!("missing required key: {key}")));
        }
    }
    for (key, _) in object {
        if !required
            .iter()
            .chain(optional.iter())
            .any(|allowed| *allowed == key)
        {
            return Err(lockfile_error(format!("unknown key: {key}")));
        }
    }
    Ok(())
}

fn require_lockfile_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
) -> Result<String, LockfileError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| lockfile_error(format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(lockfile_error(format!("{key} has an invalid length")));
    }
    Ok(value.to_owned())
}

fn lockfile_error(message: impl Into<String>) -> LockfileError {
    LockfileError {
        code: "invalid_lockfile",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleError {
    code: &'static str,
    message: String,
}

impl LifecycleError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LifecycleError {}

pub fn set_package_state_entries(
    lock: &mut [LockEntry],
    package_id: &str,
    state: PackageLifecycleState,
) -> Result<(), LifecycleError> {
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    let entry = lock
        .iter_mut()
        .find(|entry| entry.id() == &package_id)
        .ok_or_else(|| lifecycle_error("package_not_found", "package is not installed"))?;
    entry.state = state;
    Ok(())
}

pub fn mark_package_for_removal_entries(
    lock: &mut [LockEntry],
    installed_manifests: &[Manifest],
    package_id: &str,
) -> Result<(), LifecycleError> {
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    let target = lock
        .iter()
        .find(|entry| entry.id() == &package_id)
        .ok_or_else(|| lifecycle_error("package_not_found", "package is not installed"))?;
    let target_manifest = installed_manifests
        .iter()
        .find(|manifest| manifest.id() == target.id() && manifest.version() == target.version())
        .ok_or_else(|| lifecycle_error("package_not_found", "package manifest is unavailable"))?;
    if target_manifest.package_type() == &PackageType::Core {
        return Err(lifecycle_error(
            "protected_package",
            "core packages cannot be removed",
        ));
    }
    for entry in lock.iter() {
        if entry.id() == &package_id || entry.state() == &PackageLifecycleState::PendingRemove {
            continue;
        }
        let Some(dependent) = installed_manifests
            .iter()
            .find(|manifest| manifest.id() == entry.id() && manifest.version() == entry.version())
        else {
            continue;
        };
        if dependent
            .dependencies()
            .iter()
            .any(|dependency| dependency.id() == &package_id)
        {
            return Err(lifecycle_error(
                "package_in_use",
                "another installed package depends on this package",
            ));
        }
    }
    let entry = lock
        .iter_mut()
        .find(|entry| entry.id() == &package_id)
        .ok_or_else(|| lifecycle_error("package_not_found", "package is not installed"))?;
    entry.state = PackageLifecycleState::PendingRemove;
    Ok(())
}

pub fn finalize_package_removal_entries(
    lock: &mut Vec<LockEntry>,
    package_id: &str,
) -> Result<(), LifecycleError> {
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    let Some(index) = lock.iter().position(|entry| entry.id() == &package_id) else {
        return Err(lifecycle_error(
            "invalid_state",
            "package is not pending removal",
        ));
    };
    if lock[index].state() != &PackageLifecycleState::PendingRemove {
        return Err(lifecycle_error(
            "invalid_state",
            "package is not pending removal",
        ));
    }
    lock.remove(index);
    Ok(())
}

pub fn upsert_installed_lock_entry(
    lock: &mut Vec<LockEntry>,
    package_id: PackageId,
    version: String,
    manifest_sha256: HexDigest32,
) -> Result<(), LifecycleError> {
    if version.is_empty() || version.len() > MAX_VERSION_BYTES || !is_ascii_token(&version, ".+-_")
    {
        return Err(lifecycle_error(
            "invalid_state",
            "package id or lifecycle state is invalid",
        ));
    }
    let updated = LockEntry {
        id: package_id,
        version,
        manifest_sha256,
        state: PackageLifecycleState::Installed,
    };
    if let Some(entry) = lock.iter_mut().find(|entry| entry.id() == updated.id()) {
        *entry = updated;
    } else {
        lock.push(updated);
    }
    lock.sort_by(|left, right| left.id().cmp(right.id()));
    Ok(())
}

fn lifecycle_error(code: &'static str, message: impl Into<String>) -> LifecycleError {
    LifecycleError {
        code,
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    name: String,
    uncompressed_size: u64,
    directory: bool,
    encrypted: bool,
    supported: bool,
    unix_symlink: bool,
}

impl ArchiveEntry {
    pub fn file(name: &str, uncompressed_size: u64) -> Self {
        Self {
            name: name.to_owned(),
            uncompressed_size,
            directory: false,
            encrypted: false,
            supported: true,
            unix_symlink: false,
        }
    }

    pub fn directory(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            uncompressed_size: 0,
            directory: true,
            encrypted: false,
            supported: true,
            unix_symlink: false,
        }
    }

    pub fn with_unix_symlink(mut self) -> Self {
        self.unix_symlink = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveError {
    code: &'static str,
    message: String,
}

impl ArchiveError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ArchiveError {}

pub fn validate_archive_inventory(
    manifest: &Manifest,
    entries: &[ArchiveEntry],
) -> Result<(), ArchiveError> {
    if entries.len() < 3 || entries.len() > MAX_FILE_COUNT + 2 {
        return Err(archive_error(
            "invalid_archive",
            "archive entry count is outside its budget",
        ));
    }
    let mut expected_casefold =
        BTreeSet::from(["manifest.json".to_owned(), "manifest.sig".to_owned()]);
    let mut expected_files = Vec::<(String, u64)>::new();
    for file in manifest.files() {
        let archive_name = format!("payload/{}", file.path().as_str());
        if !expected_casefold.insert(archive_name.to_lowercase()) {
            return Err(archive_error(
                "unsafe_archive_path",
                "manifest paths collide on Windows",
            ));
        }
        expected_files.push((archive_name, file.size()));
    }

    let mut seen = BTreeSet::new();
    let mut total_uncompressed = 0_u64;
    for entry in entries {
        if entry.name.is_empty() || entry.encrypted || !entry.supported || entry.unix_symlink {
            return Err(archive_error(
                "invalid_archive",
                "archive contains an unsupported or executable link entry",
            ));
        }
        let mut logical_name = entry.name.as_str();
        let trimmed;
        if entry.directory && logical_name.ends_with('/') {
            trimmed = logical_name.trim_end_matches('/').to_owned();
            logical_name = &trimmed;
        }
        if logical_name != "manifest.json" && logical_name != "manifest.sig" {
            let Some(payload_path) = logical_name.strip_prefix("payload/") else {
                return Err(archive_error(
                    "unsafe_archive_path",
                    "archive path is outside payload/",
                ));
            };
            if !is_safe_relative_package_path(payload_path) {
                return Err(archive_error(
                    "unsafe_archive_path",
                    "archive path is outside payload/",
                ));
            }
        }
        if !seen.insert(logical_name.to_lowercase()) {
            return Err(archive_error(
                "unsafe_archive_path",
                "archive contains a case-insensitive duplicate path",
            ));
        }
        if entry.directory {
            continue;
        }
        if !expected_casefold.contains(&logical_name.to_lowercase()) {
            return Err(archive_error(
                "invalid_archive",
                "archive contains an undeclared file",
            ));
        }
        if let Some((_, expected_size)) =
            expected_files.iter().find(|(name, _)| name == logical_name)
        {
            if *expected_size != entry.uncompressed_size {
                return Err(archive_error(
                    "payload_mismatch",
                    "archive file size differs from manifest",
                ));
            }
        }
        if entry.uncompressed_size > MAX_FILE_BYTES
            || total_uncompressed > MAX_PAYLOAD_BYTES.saturating_sub(entry.uncompressed_size)
        {
            return Err(archive_error(
                "invalid_archive",
                "archive expands beyond its resource budget",
            ));
        }
        total_uncompressed += entry.uncompressed_size;
    }
    if seen.len() < expected_casefold.len() {
        return Err(archive_error(
            "invalid_archive",
            "archive is missing a declared payload file",
        ));
    }
    Ok(())
}

fn archive_error(code: &'static str, message: impl Into<String>) -> ArchiveError {
    ArchiveError {
        code,
        message: message.into(),
    }
}

fn manifest_error(code: &'static str, message: impl Into<String>) -> ManifestError {
    ManifestError {
        code,
        message: message.into(),
    }
}

fn compatibility_error(message: impl Into<String>) -> CompatibilityError {
    CompatibilityError {
        code: "incompatible_package",
        message: message.into(),
    }
}

fn parse_package_type(value: &str) -> Result<PackageType, ManifestError> {
    match value {
        "core" => Ok(PackageType::Core),
        "addon" => Ok(PackageType::Addon),
        "inputmethod-data" => Ok(PackageType::InputMethodData),
        "theme" => Ok(PackageType::Theme),
        "translation" => Ok(PackageType::Translation),
        _ => Err(manifest_error(
            "invalid_manifest",
            "unsupported package type",
        )),
    }
}

fn parse_dependencies(values: &[JsonValue]) -> Result<Vec<Dependency>, ManifestError> {
    if values.len() > MAX_DEPENDENCY_COUNT {
        return Err(manifest_error(
            "invalid_manifest",
            "dependencies must be a bounded array",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let object = value
            .as_object()
            .ok_or_else(|| manifest_error("invalid_manifest", "expected a JSON object"))?;
        require_object_keys(object, &["id", "version"], &[])?;
        let id = PackageId::parse(&require_string(object, "id", MAX_PACKAGE_ID_BYTES, false)?)
            .map_err(|_| manifest_error("invalid_manifest", "dependency identity is invalid"))?;
        if !ids.insert(id.as_str().to_owned()) {
            return Err(manifest_error(
                "invalid_manifest",
                "dependency identity is invalid or duplicated",
            ));
        }
        let version = require_ascii_token_string(object, "version", MAX_VERSION_BYTES, ".+-_")?;
        result.push(Dependency { id, version });
    }
    Ok(result)
}

fn parse_permissions(values: &[JsonValue]) -> Result<Vec<String>, ManifestError> {
    if values.len() > MAX_PERMISSION_COUNT {
        return Err(manifest_error(
            "invalid_manifest",
            "permissions must be a bounded array",
        ));
    }
    let mut names = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let permission = value
            .as_string()
            .ok_or_else(|| manifest_error("invalid_manifest", "permission must be a string"))?;
        if permission.len() > 64
            || !is_ascii_token(permission, "-_")
            || !names.insert(permission.to_owned())
        {
            return Err(manifest_error(
                "invalid_manifest",
                "permission is invalid or duplicated",
            ));
        }
        result.push(permission.to_owned());
    }
    Ok(result)
}

fn parse_manifest_files(
    format_version: u64,
    package_id: &PackageId,
    object: &[(String, JsonValue)],
) -> Result<Vec<VerifiedArtifact>, ManifestError> {
    let files_key = if format_version == MANIFEST_FORMAT_VERSION_V1 {
        "files"
    } else {
        "payload"
    };
    let values = require_array(object, files_key)?;
    if values.is_empty() || values.len() > MAX_FILE_COUNT {
        return Err(manifest_error(
            "invalid_manifest",
            "files must be a non-empty bounded array",
        ));
    }

    let mut exact_paths = BTreeSet::new();
    let mut windows_paths = BTreeSet::new();
    let mut total_size = 0_u64;
    let mut result = Vec::new();
    for value in values {
        let file = value
            .as_object()
            .ok_or_else(|| manifest_error("invalid_manifest", "expected a JSON object"))?;
        if format_version == MANIFEST_FORMAT_VERSION_V1 {
            require_object_keys(file, &["path", "size", "sha256"], &[])?;
        } else {
            require_object_keys(file, &["path", "size", "hashes"], &[])?;
        }

        let path = SafeRelativePackagePath::parse(&require_string(
            file,
            "path",
            MAX_PACKAGE_PATH_BYTES,
            false,
        )?)
        .map_err(|_| manifest_error("invalid_manifest", "file path is invalid"))?;
        let size = require_unsigned(file, "size", "invalid_manifest")?;
        if size > MAX_FILE_BYTES || total_size > MAX_PAYLOAD_BYTES.saturating_sub(size) {
            return Err(manifest_error(
                "invalid_manifest",
                "file entry violates path, hash or resource limits",
            ));
        }
        total_size += size;
        if !exact_paths.insert(path.as_str().to_owned())
            || !windows_paths.insert(path.as_str().to_lowercase())
        {
            return Err(manifest_error(
                "invalid_manifest",
                "file entry violates path, hash or resource limits",
            ));
        }

        let hashes = if format_version == MANIFEST_FORMAT_VERSION_V1 {
            PayloadHashes::v1_sha256(
                HexDigest32::parse(&require_string(file, "sha256", 64, false)?)
                    .map_err(|_| manifest_error("invalid_manifest", "file hash is invalid"))?,
            )
        } else {
            let hashes = require_object(file, "hashes")?;
            require_object_keys(hashes, &["blake3"], &["sha256"])?;
            let blake3 = HexDigest32::parse(&require_string(hashes, "blake3", 64, false)?)
                .map_err(|_| manifest_error("invalid_manifest", "file hash is invalid"))?;
            let sha256 = if object_contains(hashes, "sha256") {
                Some(
                    HexDigest32::parse(&require_string(hashes, "sha256", 64, false)?)
                        .map_err(|_| manifest_error("invalid_manifest", "file hash is invalid"))?,
                )
            } else {
                None
            };
            PayloadHashes::v2_blake3(blake3, sha256)
        };
        result.push(VerifiedArtifact::new(
            package_id.clone(),
            path,
            size,
            hashes,
        ));
    }
    Ok(result)
}

fn require_object_keys(
    object: &[(String, JsonValue)],
    required: &[&str],
    optional: &[&str],
) -> Result<(), ManifestError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(manifest_error(
                "invalid_manifest",
                format!("missing required key: {key}"),
            ));
        }
    }
    for (key, _) in object {
        if !required
            .iter()
            .chain(optional.iter())
            .any(|allowed| *allowed == key)
        {
            return Err(manifest_error(
                "invalid_manifest",
                format!("unknown key: {key}"),
            ));
        }
    }
    Ok(())
}

fn require_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
) -> Result<String, ManifestError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| manifest_error("invalid_manifest", format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(manifest_error(
            "invalid_manifest",
            format!("{key} has an invalid length"),
        ));
    }
    Ok(value.to_owned())
}

fn require_ascii_token_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    extra: &str,
) -> Result<String, ManifestError> {
    let value = require_string(object, key, maximum, false)?;
    if !is_ascii_token(&value, extra) {
        return Err(manifest_error(
            "invalid_manifest",
            format!("{key} is not an accepted token"),
        ));
    }
    Ok(value)
}

fn require_unsigned(
    object: &[(String, JsonValue)],
    key: &str,
    error_code: &'static str,
) -> Result<u64, ManifestError> {
    object_get(object, key)
        .and_then(JsonValue::as_number)
        .ok_or_else(|| manifest_error(error_code, format!("{key} must be an unsigned integer")))
}

fn require_array<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
) -> Result<&'a [JsonValue], ManifestError> {
    object_get(object, key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| manifest_error("invalid_manifest", format!("{key} must be an array")))
}

fn require_object<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
) -> Result<&'a [(String, JsonValue)], ManifestError> {
    object_get(object, key)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| manifest_error("invalid_manifest", format!("{key} must be an object")))
}

fn require_object_keys_with_code(
    object: &[(String, JsonValue)],
    required: &[&str],
    optional: &[&str],
    _code: &'static str,
) -> Result<(), KeyringError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(keyring_error(format!("missing required key: {key}")));
        }
    }
    for (key, _) in object {
        if !required
            .iter()
            .chain(optional.iter())
            .any(|allowed| *allowed == key)
        {
            return Err(keyring_error(format!("unknown key: {key}")));
        }
    }
    Ok(())
}

fn require_object_for_code<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
    _code: &'static str,
) -> Result<&'a [(String, JsonValue)], KeyringError> {
    object_get(object, key)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| keyring_error(format!("{key} must be an object")))
}

fn require_string_for_code(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
    _code: &'static str,
) -> Result<String, KeyringError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| keyring_error(format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(keyring_error(format!("{key} has an invalid length")));
    }
    Ok(value.to_owned())
}

fn object_contains(object: &[(String, JsonValue)], key: &str) -> bool {
    object.iter().any(|(candidate, _)| candidate == key)
}

fn object_get<'a>(object: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    object
        .iter()
        .rev()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn is_ascii_token(value: &str, extra: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || extra.as_bytes().contains(&byte))
}

fn decode_base64(value: &str) -> Result<Vec<u8>, ()> {
    if value.is_empty() || value.len() % 4 != 0 {
        return Err(());
    }
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    let chunks = value.as_bytes().chunks_exact(4);
    if !chunks.remainder().is_empty() {
        return Err(());
    }
    let chunk_count = value.len() / 4;
    for (index, chunk) in value.as_bytes().chunks_exact(4).enumerate() {
        let last = index + 1 == chunk_count;
        let a = base64_value(chunk[0]).ok_or(())?;
        let b = base64_value(chunk[1]).ok_or(())?;
        let c = if chunk[2] == b'=' {
            if !last || chunk[3] != b'=' {
                return Err(());
            }
            None
        } else {
            Some(base64_value(chunk[2]).ok_or(())?)
        };
        let d = if chunk[3] == b'=' {
            if !last {
                return Err(());
            }
            None
        } else {
            Some(base64_value(chunk[3]).ok_or(())?)
        };
        output.push((a << 2) | (b >> 4));
        if let Some(c) = c {
            output.push((b << 4) | (c >> 2));
            if let Some(d) = d {
                output.push((c << 6) | d);
            }
        }
    }
    Ok(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    String(String),
    Number(u64),
    Bool,
    Null,
}

impl JsonValue {
    fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<u64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn parse(mut self) -> Result<JsonValue, String> {
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.index != self.input.len() {
            return Err("manifest has trailing bytes".to_owned());
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(b't') => self.consume_literal("true").map(|()| JsonValue::Bool),
            Some(b'f') => self.consume_literal("false").map(|()| JsonValue::Bool),
            Some(b'n') => self.consume_literal("null").map(|()| JsonValue::Null),
            _ => Err("manifest is not strict JSON".to_owned()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect_byte(b'{')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(values));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            values.push((key, value));
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Object(values))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.expect_byte(b'[')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.next_byte() {
            match byte {
                b'"' => return Ok(output),
                b'\\' => output.push(self.parse_escape()?),
                0x00..=0x1f => return Err("manifest string contains a control byte".to_owned()),
                _ => {
                    let start = self.index - 1;
                    let character = self.input[start..]
                        .chars()
                        .next()
                        .ok_or_else(|| "manifest string is truncated".to_owned())?;
                    self.index = start + character.len_utf8();
                    output.push(character);
                }
            }
        }
        Err("manifest string is unterminated".to_owned())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        match self.next_byte() {
            Some(b'"') => Ok('"'),
            Some(b'\\') => Ok('\\'),
            Some(b'/') => Ok('/'),
            Some(b'b') => Ok('\u{0008}'),
            Some(b'f') => Ok('\u{000c}'),
            Some(b'n') => Ok('\n'),
            Some(b'r') => Ok('\r'),
            Some(b't') => Ok('\t'),
            Some(b'u') => self.parse_unicode_escape(),
            _ => Err("manifest string has an invalid escape".to_owned()),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let high = self.parse_hex_quad()?;
        if (0xd800..=0xdbff).contains(&high) {
            let checkpoint = self.index;
            if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u') {
                return Err("manifest string has an unpaired surrogate".to_owned());
            }
            let low = self.parse_hex_quad()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                self.index = checkpoint;
                return Err("manifest string has an invalid surrogate pair".to_owned());
            }
            let codepoint = 0x10000 + (((high - 0xd800) << 10) | (low - 0xdc00));
            char::from_u32(codepoint).ok_or_else(|| "manifest unicode escape is invalid".to_owned())
        } else if (0xdc00..=0xdfff).contains(&high) {
            Err("manifest string has an unpaired surrogate".to_owned())
        } else {
            char::from_u32(high).ok_or_else(|| "manifest unicode escape is invalid".to_owned())
        }
    }

    fn parse_hex_quad(&mut self) -> Result<u32, String> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self
                .next_byte()
                .ok_or_else(|| "manifest unicode escape is truncated".to_owned())?;
            value = value
                .checked_mul(16)
                .and_then(|prefix| byte_to_hex(byte).map(|digit| prefix + digit))
                .ok_or_else(|| "manifest unicode escape is invalid".to_owned())?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<u64, String> {
        let start = self.index;
        if self.consume_byte(b'0') {
            if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err("manifest number has a leading zero".to_owned());
            }
        } else {
            self.consume_digits()?;
        }
        if matches!(self.peek_byte(), Some(b'.' | b'e' | b'E')) {
            return Err("manifest number must be an unsigned integer".to_owned());
        }
        self.input[start..self.index]
            .parse::<u64>()
            .map_err(|_| "manifest number is out of range".to_owned())
    }

    fn consume_digits(&mut self) -> Result<(), String> {
        let start = self.index;
        while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.index == start {
            return Err("manifest number expected a digit".to_owned());
        }
        Ok(())
    }

    fn consume_literal(&mut self, literal: &str) -> Result<(), String> {
        if self.input[self.index..].starts_with(literal) {
            self.index += literal.len();
            Ok(())
        } else {
            Err("manifest literal is invalid".to_owned())
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err("manifest is not strict JSON".to_owned())
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.index += 1;
        Some(byte)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index).copied()
    }
}

fn byte_to_hex(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../../tests/fixtures/package_path_corpus.json");

    #[test]
    fn package_path_corpus_matches_frozen_cpp_policy() {
        let cases = parse_path_cases(CORPUS);
        assert!(cases.len() >= 20, "path corpus is unexpectedly small");
        for (path, accepted) in cases {
            assert_eq!(
                is_safe_relative_package_path(&path),
                accepted,
                "package path corpus mismatch for {path:?}"
            );
        }
    }

    #[test]
    fn case_collision_corpus_matches_frozen_cpp_policy() {
        let sets = parse_collision_sets(CORPUS);
        assert!(
            sets.len() >= 2,
            "case collision corpus is unexpectedly small"
        );
        for paths in sets {
            let parsed: Vec<_> = paths
                .iter()
                .map(|path| {
                    SafeRelativePackagePath::parse(path).expect("collision fixture path is valid")
                })
                .collect();
            assert!(
                has_windows_ordinal_case_collision(parsed.iter()),
                "case collision fixture did not collide: {paths:?}"
            );
        }
    }

    #[test]
    fn strong_types_reject_invalid_identity_and_hashes() {
        assert!(PackageId::parse("fcitx5-rime").is_ok());
        assert!(PackageId::parse("Fcitx5-Rime").is_err());
        assert!(SafeRelativePackagePath::parse("bin/addon.dll").is_ok());
        assert!(SafeRelativePackagePath::parse("bin/CON").is_err());
        assert!(HexDigest32::parse(&"a".repeat(64)).is_ok());
        assert!(HexDigest32::parse("not-a-digest").is_err());
    }

    #[test]
    fn manifest_v1_matches_cpp_golden_shape() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("valid v1 manifest should parse");

        assert_eq!(manifest.format_version(), MANIFEST_FORMAT_VERSION_V1);
        assert_eq!(manifest.id().as_str(), "fcitx5-rime");
        assert_eq!(manifest.version(), "1.0.0");
        assert_eq!(manifest.package_type(), &PackageType::Addon);
        assert_eq!(manifest.architecture(), ARCHITECTURE);
        assert_eq!(manifest.core_api(), "1");
        assert_eq!(manifest.addon_abi(), "1");
        assert_eq!(manifest.files().len(), 1);
        assert_eq!(manifest.files()[0].path().as_str(), "bin/addon.dll");
        assert_eq!(manifest.files()[0].size(), 12);
        assert!(manifest.files()[0].hashes().blake3().is_none());
        assert_eq!(
            manifest.files()[0].hashes().sha256().unwrap().as_str(),
            &"a".repeat(64)
        );
    }

    #[test]
    fn manifest_compatibility_matches_cpp_runtime_policy() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("valid manifest should parse");
        validate_manifest_compatibility(&manifest, ARCHITECTURE)
            .expect("matching architecture should be compatible");

        let any_arch = parse_manifest(
            &manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12).replace(
                &format!("\"architecture\":\"{ARCHITECTURE}\""),
                "\"architecture\":\"any\"",
            ),
        )
        .expect("any architecture manifest should parse");
        validate_manifest_compatibility(&any_arch, ARCHITECTURE)
            .expect("any architecture should be compatible");

        let opposite_architecture = if ARCHITECTURE == "x64" { "x86" } else { "x64" };
        assert_compatibility_error(&manifest, opposite_architecture);
        assert_compatibility_error(&manifest, "arm64");

        let unsupported_core_api = parse_manifest(
            &manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
                .replace("\"core_api\":\"1\"", "\"core_api\":\"2\""),
        )
        .expect("manifest with unsupported API should still parse");
        assert_compatibility_error(&unsupported_core_api, ARCHITECTURE);

        let unsupported_addon_abi = parse_manifest(
            &manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
                .replace("\"addon_abi\":\"1\"", "\"addon_abi\":\"2\""),
        )
        .expect("manifest with unsupported ABI should still parse");
        assert_compatibility_error(&unsupported_addon_abi, ARCHITECTURE);

        let core_without_addon_abi = parse_manifest(
            &manifest_v1("fcitx5-core", "1.0.0", &"c".repeat(64), 12)
                .replace("\"type\":\"addon\"", "\"type\":\"core\"")
                .replace("\"addon_abi\":\"1\"", "\"addon_abi\":\"\""),
        )
        .expect("core manifest should parse");
        validate_manifest_compatibility(&core_without_addon_abi, ARCHITECTURE)
            .expect("core package should not require addon ABI");
    }

    #[test]
    fn manifest_v2_requires_blake3_and_accepts_optional_sha256() {
        let manifest = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            &"b".repeat(64),
            12,
            "official-2026-mldsa65",
            Some(&"a".repeat(64)),
        ))
        .expect("valid v2 manifest should parse");

        assert_eq!(manifest.format_version(), MANIFEST_FORMAT_VERSION_V2);
        assert_eq!(manifest.key_id().as_str(), "official-2026-mldsa65");
        assert_eq!(
            manifest.files()[0].hashes().blake3().unwrap().as_str(),
            &"b".repeat(64)
        );
        assert_eq!(
            manifest.files()[0].hashes().sha256().unwrap().as_str(),
            &"a".repeat(64)
        );

        let missing_blake3 = manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            &"b".repeat(64),
            12,
            "official-2026-mldsa65",
            None,
        )
        .replace("\"blake3\":\"", "\"not_blake3\":\"");
        assert_manifest_error("invalid_manifest", &missing_blake3);
    }

    #[test]
    fn manifest_rejects_cpp_invalid_cases() {
        let unsupported = manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
            .replace("\"format_version\": 1", "\"format_version\": 99");
        assert_manifest_error("unsupported_manifest", &unsupported);

        let bad_hash = manifest_v1("fcitx5-rime", "1.0.0", "not-a-digest", 12);
        assert_manifest_error("invalid_manifest", &bad_hash);

        let traversal = manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
            .replace("bin/addon.dll", "../escape.dll");
        assert_manifest_error("invalid_manifest", &traversal);

        let duplicate_dependency =
            manifest_v1_with_dependencies("[{\"id\":\"fcitx5-rime\",\"version\":\"1\"},{\"id\":\"fcitx5-rime\",\"version\":\"1\"}]");
        assert_manifest_error("invalid_manifest", &duplicate_dependency);

        let case_collision = manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12).replace(
            "{\"path\":\"bin/addon.dll\",\"size\":12,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
            "{\"path\":\"bin/addon.dll\",\"size\":12,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"},{\"path\":\"BIN/ADDON.DLL\",\"size\":12,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
        );
        assert_manifest_error("invalid_manifest", &case_collision);
    }

    #[test]
    fn trusted_keyring_v2_matches_cpp_pqc_schema() {
        let mldsa = vec![0x41; MLDSA65_PUBLIC_KEY_BYTES];
        let slhdsa = vec![0x42; SLHDSA_SHA2_128S_PUBLIC_KEY_BYTES];
        let keyring = keyring_v2(&mldsa, &slhdsa);
        let keys = parse_trusted_keys(&keyring).expect("valid PQC keyring should parse");

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].id().as_str(), "official-2026-mldsa65");
        assert_eq!(keys[0].algorithm(), &TrustAlgorithm::Mldsa65);
        assert_eq!(keys[0].public_key().len(), MLDSA65_PUBLIC_KEY_BYTES);
        assert!(!keys[0].revoked());
        assert!(keys[1].revoked());
        assert_eq!(keys[2].algorithm(), &TrustAlgorithm::SlhdsaSha2_128s);

        let bad_mldsa_length =
            keyring.replacen(&base64_for_test(&mldsa), &base64_for_test(&slhdsa), 1);
        assert_keyring_error(&bad_mldsa_length);

        let duplicate = keyring.replace("official-2026-mldsa65-revoked", "official-2026-mldsa65");
        assert_keyring_error(&duplicate);

        let unsupported_required = keyring.replacen("\"mldsa65\"", "\"ed25519\"", 1);
        assert_keyring_error(&unsupported_required);
    }

    #[test]
    fn signature_envelope_v2_matches_cpp_schema() {
        let mldsa_signature = vec![0x43; 3309];
        let slhdsa_signature = vec![0x44; 7856];
        let index = signature_envelope(
            "repository-index",
            &[
                (
                    "official-2026-mldsa65",
                    "mldsa65",
                    mldsa_signature.as_slice(),
                ),
                (
                    "official-2026-slh-dsa-recovery",
                    "slhdsa-sha2-128s",
                    slhdsa_signature.as_slice(),
                ),
            ],
        );
        let parsed = parse_signature_envelope(&index, SignedObject::RepositoryIndex)
            .expect("repository index envelope should parse");
        assert_eq!(parsed.format_version(), 2);
        assert_eq!(parsed.signed_object(), &SignedObject::RepositoryIndex);
        assert_eq!(
            parsed.canonicalization(),
            SIGNATURE_ENVELOPE_CANONICALIZATION
        );
        assert_eq!(parsed.signatures().len(), 2);
        assert_eq!(parsed.signatures()[0].algorithm(), &TrustAlgorithm::Mldsa65);
        assert_eq!(parsed.signatures()[0].signature().len(), 3309);

        assert_signature_error(&index, SignedObject::PackageManifest);

        let manifest = signature_envelope(
            "package-manifest",
            &[(
                "official-2026-mldsa65",
                "mldsa65",
                mldsa_signature.as_slice(),
            )],
        );
        assert!(
            parse_signature_envelope(&manifest, SignedObject::PackageManifest).is_ok(),
            "package manifest envelope should parse"
        );

        let missing_mldsa = signature_envelope(
            "package-manifest",
            &[(
                "official-2026-slh-dsa-recovery",
                "slhdsa-sha2-128s",
                slhdsa_signature.as_slice(),
            )],
        );
        assert_signature_error(&missing_mldsa, SignedObject::PackageManifest);

        let unsupported = manifest.replacen("\"mldsa65\"", "\"ed25519\"", 1);
        assert_signature_error(&unsupported, SignedObject::PackageManifest);

        let duplicate = index.replace("official-2026-slh-dsa-recovery", "official-2026-mldsa65");
        assert_signature_error(&duplicate, SignedObject::RepositoryIndex);

        let malformed = manifest.replace(&base64_for_test(&mldsa_signature), "not base64!");
        assert_signature_error(&malformed, SignedObject::PackageManifest);
    }

    #[test]
    fn dependency_resolution_matches_cpp_exact_ordering() {
        let rime = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("rime manifest should parse");
        let schema = parse_manifest(&manifest_v1_with_dependencies_for(
            "rime-schema-luna",
            "1.0.0",
            &"b".repeat(64),
            12,
            "[{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\"}]",
        ))
        .expect("schema manifest should parse");
        let order =
            resolve_exact_dependencies(&[schema.clone(), rime.clone()], &["rime-schema-luna"])
                .expect("dependency resolution should succeed");
        assert_eq!(order, vec!["fcitx5-rime", "rime-schema-luna"]);

        let missing_version = parse_manifest(&manifest_v1_with_dependencies_for(
            "rime-schema-luna",
            "1.0.0",
            &"b".repeat(64),
            12,
            "[{\"id\":\"fcitx5-rime\",\"version\":\"2.0.0\"}]",
        ))
        .expect("schema manifest should parse");
        assert_resolution_error(&[missing_version, rime.clone()], &["rime-schema-luna"]);
        assert_resolution_error(&[rime.clone(), rime.clone()], &["fcitx5-rime"]);

        let cycle_a = parse_manifest(&manifest_v1_with_dependencies_for(
            "cycle-a",
            "1.0.0",
            &"c".repeat(64),
            12,
            "[{\"id\":\"cycle-b\",\"version\":\"1.0.0\"}]",
        ))
        .expect("cycle manifest should parse");
        let cycle_b = parse_manifest(&manifest_v1_with_dependencies_for(
            "cycle-b",
            "1.0.0",
            &"d".repeat(64),
            12,
            "[{\"id\":\"cycle-a\",\"version\":\"1.0.0\"}]",
        ))
        .expect("cycle manifest should parse");
        assert_resolution_error(&[cycle_a, cycle_b], &["cycle-a"]);
    }

    #[test]
    fn payload_inventory_matches_cpp_declared_file_contract() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let declared = PayloadEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
        );
        assert!(verify_payload_inventory(&manifest, &[declared.clone()]).is_ok());

        let missing: [PayloadEntry; 0] = [];
        assert_payload_error(&manifest, &missing);
        let wrong_size = PayloadEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            13,
        );
        assert_payload_error(&manifest, &[wrong_size]);
        let extra = PayloadEntry::new(
            SafeRelativePackagePath::parse("bin/extra.dll").expect("path should parse"),
            12,
        );
        assert_payload_error(&manifest, &[declared.clone(), extra]);
        let case_variant = PayloadEntry::new(
            SafeRelativePackagePath::parse("BIN/ADDON.DLL").expect("path should parse"),
            12,
        );
        assert_payload_error(&manifest, &[declared, case_variant]);
    }

    #[test]
    fn payload_digest_matching_keeps_cpp_v1_v2_hash_semantics() {
        let manifest_v1 = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let v1_observed = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            None,
            Some(HexDigest32::parse(&"a".repeat(64)).expect("hash should parse")),
        );
        assert!(verify_payload_digests(&manifest_v1, &[v1_observed]).is_ok());
        let v1_bad = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            None,
            Some(HexDigest32::parse(&"b".repeat(64)).expect("hash should parse")),
        );
        assert_payload_digest_error(&manifest_v1, &[v1_bad]);

        let manifest_v2 = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "2.0.0",
            &"c".repeat(64),
            12,
            "official-2026-mldsa65",
            Some(&"d".repeat(64)),
        ))
        .expect("manifest should parse");
        let v2_observed = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            Some(HexDigest32::parse(&"c".repeat(64)).expect("hash should parse")),
            Some(HexDigest32::parse(&"d".repeat(64)).expect("hash should parse")),
        );
        assert!(verify_payload_digests(&manifest_v2, &[v2_observed]).is_ok());
        let v2_bad_blake3 = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            Some(HexDigest32::parse(&"e".repeat(64)).expect("hash should parse")),
            Some(HexDigest32::parse(&"d".repeat(64)).expect("hash should parse")),
        );
        assert_payload_digest_error(&manifest_v2, &[v2_bad_blake3]);
        let v2_bad_sha = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            Some(HexDigest32::parse(&"c".repeat(64)).expect("hash should parse")),
            Some(HexDigest32::parse(&"e".repeat(64)).expect("hash should parse")),
        );
        assert_payload_digest_error(&manifest_v2, &[v2_bad_sha]);
    }

    #[test]
    fn lockfile_parser_matches_cpp_lifecycle_schema() {
        let lockfile = format!(
            "{{\"format_version\":1,\"packages\":[\
             {{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\",\
             \"manifest_sha256\":\"{}\",\"state\":\"installed\"}},\
             {{\"id\":\"rime-schema-luna\",\"version\":\"1.0.0\",\
             \"manifest_sha256\":\"{}\",\"state\":\"pending_remove\"}}]}}",
            "a".repeat(64),
            "b".repeat(64)
        );
        let entries = parse_lockfile(&lockfile).expect("valid lockfile should parse");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id().as_str(), "fcitx5-rime");
        assert_eq!(entries[0].version(), "1.0.0");
        assert_eq!(entries[0].manifest_sha256().as_str(), &"a".repeat(64));
        assert_eq!(entries[0].state(), &PackageLifecycleState::Installed);
        assert_eq!(entries[1].state().as_str(), "pending_remove");

        assert_lockfile_error(&lockfile.replace("\"format_version\":1", "\"format_version\":2"));
        assert_lockfile_error(&lockfile.replace("\"installed\"", "\"unknown\""));
        assert_lockfile_error(&lockfile.replace(&"a".repeat(64), "not-a-digest"));
        assert_lockfile_error(&lockfile.replace("rime-schema-luna", "fcitx5-rime"));
        assert_lockfile_error(
            &lockfile.replace("\"version\":\"1.0.0\"", "\"version\":\"bad version\""),
        );
    }

    #[test]
    fn archive_inventory_matches_cpp_payload_entry_policy() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let valid = [
            ArchiveEntry::file("manifest.json", 100),
            ArchiveEntry::file("manifest.sig", 64),
            ArchiveEntry::file("payload/bin/addon.dll", 12),
        ];
        assert!(validate_archive_inventory(&manifest, &valid).is_ok());

        assert_archive_error(
            "invalid_archive",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
            ],
        );
        assert_archive_error(
            "unsafe_archive_path",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12),
                ArchiveEntry::file("payload/../escape.dll", 6),
            ],
        );
        assert_archive_error(
            "unsafe_archive_path",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12),
                ArchiveEntry::file("payload/BIN/ADDON.DLL", 12),
            ],
        );
        assert_archive_error(
            "invalid_archive",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12),
                ArchiveEntry::file("payload/bin/extra.dll", 12),
            ],
        );
        assert_archive_error(
            "payload_mismatch",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 13),
            ],
        );
        assert_archive_error(
            "invalid_archive",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12).with_unix_symlink(),
            ],
        );
    }

    #[test]
    fn lifecycle_state_machine_matches_cpp_lock_rules() {
        let rime = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let schema = parse_manifest(&manifest_v1_with_dependencies_for(
            "rime-schema-luna",
            "1.0.0",
            &"b".repeat(64),
            12,
            "[{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\"}]",
        ))
        .expect("manifest should parse");
        let mut lock = vec![
            lock_entry("fcitx5-rime", PackageLifecycleState::Installed),
            lock_entry("rime-schema-luna", PackageLifecycleState::Installed),
        ];

        set_package_state_entries(&mut lock, "fcitx5-rime", PackageLifecycleState::Disabled)
            .expect("state update should succeed");
        assert_eq!(lock[0].state(), &PackageLifecycleState::Disabled);
        assert_lifecycle_error(
            "package_not_found",
            set_package_state_entries(
                &mut lock,
                "missing-package",
                PackageLifecycleState::Disabled,
            ),
        );

        assert_lifecycle_error(
            "package_in_use",
            mark_package_for_removal_entries(
                &mut lock,
                &[rime.clone(), schema.clone()],
                "fcitx5-rime",
            ),
        );
        mark_package_for_removal_entries(&mut lock, &[rime, schema], "rime-schema-luna")
            .expect("leaf package removal should be marked");
        assert_eq!(lock[1].state(), &PackageLifecycleState::PendingRemove);
        assert_lifecycle_error(
            "invalid_state",
            finalize_package_removal_entries(&mut lock.clone(), "fcitx5-rime"),
        );
        finalize_package_removal_entries(&mut lock, "rime-schema-luna")
            .expect("pending removal should finalize");
        assert_eq!(lock.len(), 1);

        let core = parse_manifest(
            &manifest_v1("fcitx5-core", "1.0.0", &"c".repeat(64), 12)
                .replace("\"type\":\"addon\"", "\"type\":\"core\""),
        )
        .expect("core manifest should parse");
        let mut core_lock = vec![lock_entry("fcitx5-core", PackageLifecycleState::Installed)];
        assert_lifecycle_error(
            "protected_package",
            mark_package_for_removal_entries(&mut core_lock, &[core], "fcitx5-core"),
        );
    }

    #[test]
    fn installed_lock_upsert_matches_cpp_activation_sorting() {
        let mut lock = vec![
            lock_entry("fcitx5-rime", PackageLifecycleState::Disabled),
            lock_entry("rime-schema-luna", PackageLifecycleState::Installed),
        ];
        upsert_installed_lock_entry(
            &mut lock,
            PackageId::parse("fcitx5-rime").expect("package id should parse"),
            "2.0.0".to_owned(),
            HexDigest32::parse(&"b".repeat(64)).expect("digest should parse"),
        )
        .expect("existing package should be replaced");

        assert_eq!(lock.len(), 2);
        assert_eq!(lock[0].id().as_str(), "fcitx5-rime");
        assert_eq!(lock[0].version(), "2.0.0");
        assert_eq!(lock[0].state(), &PackageLifecycleState::Installed);
        assert_eq!(lock[0].manifest_sha256().as_str(), &"b".repeat(64));

        upsert_installed_lock_entry(
            &mut lock,
            PackageId::parse("anthy").expect("package id should parse"),
            "1.0.0".to_owned(),
            HexDigest32::parse(&"c".repeat(64)).expect("digest should parse"),
        )
        .expect("new package should be inserted");
        assert_eq!(
            lock.iter()
                .map(|entry| entry.id().as_str())
                .collect::<Vec<_>>(),
            vec!["anthy", "fcitx5-rime", "rime-schema-luna"]
        );

        assert_lifecycle_error(
            "invalid_state",
            upsert_installed_lock_entry(
                &mut lock,
                PackageId::parse("bad-version").expect("package id should parse"),
                "bad version".to_owned(),
                HexDigest32::parse(&"d".repeat(64)).expect("digest should parse"),
            ),
        );
    }

    #[cfg(target_pointer_width = "64")]
    const ARCHITECTURE: &str = "x64";
    #[cfg(not(target_pointer_width = "64"))]
    const ARCHITECTURE: &str = "x86";

    fn manifest_v1(id: &str, version: &str, sha256: &str, size: u64) -> String {
        manifest_v1_with_dependencies_for(id, version, sha256, size, "[]")
    }

    fn manifest_v1_with_dependencies(dependencies: &str) -> String {
        manifest_v1_with_dependencies_for("fcitx5-rime", "1.0.0", &"a".repeat(64), 12, dependencies)
    }

    fn manifest_v1_with_dependencies_for(
        id: &str,
        version: &str,
        sha256: &str,
        size: u64,
        dependencies: &str,
    ) -> String {
        format!(
            "{{\"format_version\": 1,\"id\":\"{id}\",\"version\":\"{version}\",\"type\":\"addon\",\
             \"architecture\":\"{ARCHITECTURE}\",\"min_os\":\"6.1-sp1\",\"core_api\":\"1\",\
             \"addon_abi\":\"1\",\"dependencies\":{dependencies},\"license\":\"MIT\",\
             \"source_commit\":\"0123456789abcdef\",\"permissions\":[\"native-code\",\"input-data\"],\
             \"files\":[{{\"path\":\"bin/addon.dll\",\"size\":{size},\"sha256\":\"{sha256}\"}}],\
             \"key_id\":\"release-2026\"}}"
        )
    }

    fn manifest_v2(
        id: &str,
        version: &str,
        blake3: &str,
        size: u64,
        key_id: &str,
        sha256: Option<&str>,
    ) -> String {
        let sha256 = sha256.map_or(String::new(), |value| format!(",\"sha256\":\"{value}\""));
        format!(
            "{{\"format_version\": 2,\"id\":\"{id}\",\"version\":\"{version}\",\"type\":\"addon\",\
             \"architecture\":\"{ARCHITECTURE}\",\"min_os\":\"6.1-sp1\",\"core_api\":\"1\",\
             \"addon_abi\":\"1\",\"dependencies\":[],\"license\":\"MIT\",\
             \"source_commit\":\"0123456789abcdef\",\"permissions\":[\"native-code\",\"input-data\"],\
             \"payload\":[{{\"path\":\"bin/addon.dll\",\"size\":{size},\
             \"hashes\":{{\"blake3\":\"{blake3}\"{sha256}}}}}],\"key_id\":\"{key_id}\"}}"
        )
    }

    fn assert_manifest_error(code: &str, manifest: &str) {
        let error = parse_manifest(manifest).expect_err("manifest should be rejected");
        assert_eq!(error.code(), code);
    }

    fn assert_compatibility_error(manifest: &Manifest, architecture: &str) {
        assert_eq!(
            validate_manifest_compatibility(manifest, architecture)
                .expect_err("manifest should be incompatible")
                .code(),
            "incompatible_package"
        );
    }

    fn keyring_v2(mldsa: &[u8], slhdsa: &[u8]) -> String {
        format!(
            "{{\"format_version\":2,\
             \"policy\":{{\"official_required_signatures\":[\"mldsa65\"],\
             \"compatibility_hashes\":[\"sha256\"],\"default_payload_hash\":\"blake3\"}},\
             \"keys\":[\
             {{\"key_id\":\"official-2026-mldsa65\",\"algorithm\":\"mldsa65\",\
             \"status\":\"trusted\",\"public_key_base64\":\"{}\",\
             \"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]}},\
             {{\"key_id\":\"official-2026-mldsa65-revoked\",\"algorithm\":\"mldsa65\",\
             \"status\":\"revoked\",\"public_key_base64\":\"{}\",\
             \"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]}},\
             {{\"key_id\":\"official-2026-slh-dsa-recovery\",\
             \"algorithm\":\"slhdsa-sha2-128s\",\"status\":\"trusted\",\
             \"public_key_base64\":\"{}\",\"scope\":[\"repository\"],\
             \"channels\":[\"stable\"]}}]}}",
            base64_for_test(mldsa),
            base64_for_test(mldsa),
            base64_for_test(slhdsa)
        )
    }

    fn signature_envelope(signed_object: &str, signatures: &[(&str, &str, &[u8])]) -> String {
        let entries: Vec<_> = signatures
            .iter()
            .map(|(key_id, algorithm, signature)| {
                format!(
                    "{{\"key_id\":\"{key_id}\",\"algorithm\":\"{algorithm}\",\
                     \"signature_base64\":\"{}\"}}",
                    base64_for_test(signature)
                )
            })
            .collect();
        format!(
            "{{\"format_version\":2,\"signed_object\":\"{signed_object}\",\
             \"canonicalization\":\"{SIGNATURE_ENVELOPE_CANONICALIZATION}\",\
             \"signatures\":[{}]}}",
            entries.join(",")
        )
    }

    fn assert_keyring_error(keyring: &str) {
        assert_eq!(
            parse_trusted_keys(keyring)
                .expect_err("keyring should be rejected")
                .code(),
            "invalid_keyring"
        );
    }

    fn assert_signature_error(envelope: &str, expected: SignedObject) {
        assert_eq!(
            parse_signature_envelope(envelope, expected)
                .expect_err("signature envelope should be rejected")
                .code(),
            "invalid_signature"
        );
    }

    fn assert_resolution_error(available: &[Manifest], requested_ids: &[&str]) {
        assert_eq!(
            resolve_exact_dependencies(available, requested_ids)
                .expect_err("resolution should fail")
                .code(),
            "resolution_failed"
        );
    }

    fn assert_payload_error(manifest: &Manifest, observed: &[PayloadEntry]) {
        assert_eq!(
            verify_payload_inventory(manifest, observed)
                .expect_err("payload inventory should be rejected")
                .code(),
            "payload_mismatch"
        );
    }

    fn assert_payload_digest_error(manifest: &Manifest, observed: &[PayloadDigestEntry]) {
        assert_eq!(
            verify_payload_digests(manifest, observed)
                .expect_err("payload digest should be rejected")
                .code(),
            "payload_mismatch"
        );
    }

    fn assert_lockfile_error(lockfile: &str) {
        assert_eq!(
            parse_lockfile(lockfile)
                .expect_err("lockfile should be rejected")
                .code(),
            "invalid_lockfile"
        );
    }

    fn assert_archive_error(code: &str, manifest: &Manifest, entries: &[ArchiveEntry]) {
        assert_eq!(
            validate_archive_inventory(manifest, entries)
                .expect_err("archive inventory should be rejected")
                .code(),
            code
        );
    }

    fn assert_lifecycle_error(result_code: &str, result: Result<(), LifecycleError>) {
        assert_eq!(
            result.expect_err("lifecycle transition should fail").code(),
            result_code
        );
    }

    fn lock_entry(id: &str, state: PackageLifecycleState) -> LockEntry {
        LockEntry::new(
            PackageId::parse(id).expect("id should parse"),
            "1.0.0".to_owned(),
            HexDigest32::parse(&"a".repeat(64)).expect("hash should parse"),
            state,
        )
    }

    fn base64_for_test(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            output.push(ALPHABET[(b0 >> 2) as usize] as char);
            output.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                output.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                output.push('=');
            }
            if chunk.len() > 2 {
                output.push(ALPHABET[(b2 & 0x3f) as usize] as char);
            } else {
                output.push('=');
            }
        }
        output
    }

    fn parse_path_cases(corpus: &str) -> Vec<(String, bool)> {
        corpus
            .lines()
            .filter_map(|line| {
                let path_start = line.find("\"path\": \"")? + "\"path\": \"".len();
                let path_end = find_json_string_end(line, path_start);
                let accepted_start = line.find("\"accepted\": ")? + "\"accepted\": ".len();
                let accepted = line[accepted_start..].starts_with("true");
                Some((unescape_json_string(&line[path_start..path_end]), accepted))
            })
            .collect()
    }

    fn parse_collision_sets(corpus: &str) -> Vec<Vec<String>> {
        corpus
            .lines()
            .filter_map(|line| {
                let paths_start = line.find("\"paths\": [")? + "\"paths\": [".len();
                let paths_end = line[paths_start..].find(']')? + paths_start;
                let inner = &line[paths_start..paths_end];
                let mut paths = Vec::new();
                let mut rest = inner;
                while let Some(start) = rest.find('"') {
                    let value_start = start + 1;
                    let value_end = find_json_string_end(rest, value_start);
                    paths.push(unescape_json_string(&rest[value_start..value_end]));
                    rest = &rest[value_end + 1..];
                }
                Some(paths)
            })
            .collect()
    }

    fn find_json_string_end(text: &str, start: usize) -> usize {
        let bytes = text.as_bytes();
        let mut index = start;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                return index;
            }
            index += 1;
        }
        panic!("unterminated JSON string in test corpus");
    }

    fn unescape_json_string(value: &str) -> String {
        let mut output = String::new();
        let mut chars = value.chars();
        while let Some(character) = chars.next() {
            if character != '\\' {
                output.push(character);
                continue;
            }
            match chars.next().expect("dangling JSON escape") {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                '/' => output.push('/'),
                'b' => output.push('\u{0008}'),
                'f' => output.push('\u{000c}'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'u' => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        hex.push(chars.next().expect("short JSON unicode escape"));
                    }
                    let codepoint =
                        u32::from_str_radix(&hex, 16).expect("invalid JSON unicode escape");
                    output.push(char::from_u32(codepoint).expect("invalid JSON unicode codepoint"));
                }
                other => panic!("unsupported JSON escape: {other}"),
            }
        }
        output
    }
}
