use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use ring::{digest, signature};
use semver::Version;
use serde::Deserialize;
use ureq::tls::{RootCerts, TlsConfig};

use super::{Error, MAX_PACKAGE, MAX_SUMS, SIGNATURE, SUMS, hex_bytes, package_name, public_key};

const REPOSITORY: &str = "MikeNoppo/solideoworship-app";
const RELEASES: &str = "https://api.github.com/repos/MikeNoppo/solideoworship-app/releases";
const MAX_RELEASE_PAGE: u64 = 8 * 1024 * 1024;
const MAX_PAGES: u8 = 10;
const PAGE_SIZE: usize = 30;
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);

#[derive(Debug, Clone)]
pub struct ReleaseUpdate {
    pub version: Version,
    size: u64,
    sums: Arc<[u8]>,
    signature: Arc<[u8]>,
    checksum: [u8; 32],
}

#[derive(Deserialize)]
struct Release {
    #[serde(rename = "tag_name")]
    tag: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    state: String,
}

impl Release {
    fn asset(&self, name: &str, max_bytes: u64) -> Option<&Asset> {
        self.assets.iter().find(|asset| {
            asset.name == name && asset.state == "uploaded" && (1..=max_bytes).contains(&asset.size)
        })
    }
}

pub fn check() -> Result<Option<ReleaseUpdate>, Error> {
    let key = public_key().ok_or(Error::Verification)?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| Error::Metadata)?;
    let agent = agent(CHECK_TIMEOUT);
    let mut best = None;
    for page in 1..=MAX_PAGES {
        let url = format!("{RELEASES}?page={page}&per_page={PAGE_SIZE}");
        let bytes = read_limited(
            &agent,
            &url,
            MAX_RELEASE_PAGE,
            "application/vnd.github+json",
        )?;
        let releases: Vec<Release> = serde_json::from_slice(&bytes).map_err(|_| Error::Metadata)?;
        if let Some((release, version)) = choose(&releases, &current)
            && best
                .as_ref()
                .is_none_or(|(best, _)| version.cmp_precedence(best).is_gt())
        {
            let size = release
                .asset(&package_name(&version.to_string()), MAX_PACKAGE)
                .ok_or(Error::Metadata)?
                .size;
            best = Some((version, size));
        }

        if releases.len() < PAGE_SIZE {
            return best
                .map(|(version, size)| verify_release(&agent, key, version, size))
                .transpose();
        }
    }
    Err(Error::Metadata)
}

fn choose<'a>(releases: &'a [Release], current: &Version) -> Option<(&'a Release, Version)> {
    releases
        .iter()
        .filter_map(|release| {
            let version = Version::parse(release.tag.strip_prefix('v')?).ok()?;
            let prerelease = !version.pre.is_empty();
            if !eligible(current, &version) || release.draft || release.prerelease != prerelease {
                return None;
            }
            release.asset(&package_name(&version.to_string()), MAX_PACKAGE)?;
            release.asset(SUMS, MAX_SUMS)?;
            release.asset(SIGNATURE, 64)?;
            Some((release, version))
        })
        .max_by(|(_, left), (_, right)| left.cmp_precedence(right))
}

pub(super) fn eligible(current: &Version, candidate: &Version) -> bool {
    candidate.cmp_precedence(current).is_gt()
        && (candidate.pre.is_empty()
            || (!current.pre.is_empty()
                && candidate.pre.as_str().split('.').next()
                    == current.pre.as_str().split('.').next()))
}

fn verify_release(
    agent: &ureq::Agent,
    key: [u8; 32],
    version: Version,
    size: u64,
) -> Result<ReleaseUpdate, Error> {
    let sums = read_limited(
        agent,
        &asset_url(&version, SUMS),
        MAX_SUMS,
        "application/octet-stream",
    )?;
    let signature = read_limited(
        agent,
        &asset_url(&version, SIGNATURE),
        64,
        "application/octet-stream",
    )?;
    let checksum = signed_checksum(key, &package_name(&version.to_string()), &sums, &signature)?;
    Ok(ReleaseUpdate {
        version,
        size,
        sums: sums.into(),
        signature: signature.into(),
        checksum,
    })
}

impl ReleaseUpdate {
    pub(super) fn download(&self, directory: &Path) -> Result<(), Error> {
        let name = package_name(&self.version.to_string());
        let mut response = agent(DOWNLOAD_TIMEOUT)
            .get(asset_url(&self.version, &name))
            .call()
            .map_err(|_| Error::Network)?;
        let mut file = File::create_new(directory.join(name)).map_err(|_| Error::Install)?;
        copy_verified(
            response.body_mut().as_reader(),
            &mut file,
            Some(self.size),
            self.checksum,
        )?;
        file.sync_all().map_err(|_| Error::Install)?;
        fs::write(directory.join(SUMS), &self.sums).map_err(|_| Error::Install)?;
        fs::write(directory.join(SIGNATURE), &self.signature).map_err(|_| Error::Install)?;
        Ok(())
    }
}

#[cfg(any(target_os = "linux", windows))]
pub(super) fn verify_staged(directory: &Path, name: &str, file: impl Read) -> Result<(), Error> {
    let key = public_key().ok_or(Error::Verification)?;
    let mut sums = Vec::new();
    let mut sig = Vec::new();
    File::open(directory.join(SUMS))
        .and_then(|file| file.take(MAX_SUMS + 1).read_to_end(&mut sums))
        .map_err(|_| Error::Verification)?;
    File::open(directory.join(SIGNATURE))
        .and_then(|file| file.take(65).read_to_end(&mut sig))
        .map_err(|_| Error::Verification)?;
    if sums.len() as u64 > MAX_SUMS {
        return Err(Error::Verification);
    }
    let checksum = signed_checksum(key, name, &sums, &sig)?;
    copy_verified(file, io::sink(), None, checksum)
}

fn signed_checksum(key: [u8; 32], name: &str, sums: &[u8], sig: &[u8]) -> Result<[u8; 32], Error> {
    signature::UnparsedPublicKey::new(&signature::ED25519, key)
        .verify(sums, sig)
        .map_err(|_| Error::Verification)?;
    let text = std::str::from_utf8(sums).map_err(|_| Error::Verification)?;
    let mut hashes = text.lines().filter_map(|line| {
        let (hash, file) = line.split_once("  ")?;
        (file == name).then_some(hash)
    });
    let hash = hashes
        .next()
        .and_then(hex_bytes)
        .ok_or(Error::Verification)?;
    if hashes.next().is_some() {
        return Err(Error::Verification);
    }
    Ok(hash)
}

fn copy_verified(
    mut reader: impl Read,
    mut writer: impl Write,
    size: Option<u64>,
    expected: [u8; 32],
) -> Result<(), Error> {
    let mut bytes = 0;
    let mut hash = digest::Context::new(&digest::SHA256);
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|_| Error::Network)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > size.unwrap_or(MAX_PACKAGE) {
            return Err(Error::Verification);
        }
        hash.update(&buffer[..count]);
        writer
            .write_all(&buffer[..count])
            .map_err(|_| Error::Install)?;
    }
    if size.is_some_and(|size| size != bytes) || hash.finish().as_ref() != expected {
        return Err(Error::Verification);
    }
    Ok(())
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(3)
        .timeout_connect(Some(CHECK_TIMEOUT))
        .timeout_global(Some(timeout))
        .user_agent("SolideoWorship-Updater")
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .unversioned_rustls_crypto_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .build(),
        )
        .build()
        .into()
}

fn asset_url(version: &Version, name: &str) -> String {
    format!("https://github.com/{REPOSITORY}/releases/download/v{version}/{name}")
}

fn read_limited(
    agent: &ureq::Agent,
    url: &str,
    limit: u64,
    accept: &str,
) -> Result<Vec<u8>, Error> {
    let mut response = agent
        .get(url)
        .header("Accept", accept)
        .call()
        .map_err(|_| Error::Network)?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Network)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Metadata);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn release(tag: &str, draft: bool, prerelease: bool, has_package: bool) -> Release {
        let mut names = vec![SUMS.into(), SIGNATURE.into()];
        if has_package {
            names.push(package_name(tag.strip_prefix('v').unwrap()));
        }
        Release {
            tag: tag.into(),
            draft,
            prerelease,
            assets: names
                .into_iter()
                .map(|name| Asset {
                    name,
                    size: 1,
                    state: "uploaded".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn chooses_newer_versions_with_the_matching_platform_and_channel() {
        let mut releases = vec![
            release("v0.1.0-beta.8", false, true, true),
            release("v0.1.0-beta.10", false, true, true),
            release("v0.1.0-beta.99", true, true, true),
            release("v1.0.0", false, false, false),
        ];

        let beta = Version::parse("0.1.0-beta.9").unwrap();
        assert_eq!(
            choose(&releases, &beta).unwrap().1.to_string(),
            "0.1.0-beta.10"
        );

        releases.push(release("v0.1.0", false, false, true));
        assert_eq!(choose(&releases, &beta).unwrap().1.to_string(), "0.1.0");

        releases.push(release("v1.0.0-beta.1", false, true, true));
        let stable = Version::parse("0.1.0").unwrap();
        assert!(choose(&releases, &stable).is_none());
    }

    #[test]
    fn requires_signature_and_an_exact_unique_checksum() {
        let key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        let public = key.public_key().as_ref().try_into().unwrap();
        let name = package_name("1.0.0");
        let hash = "ab".repeat(32);
        let sums = format!("{hash}  {name}\n");
        let signature = key.sign(sums.as_bytes());
        assert_eq!(
            signed_checksum(public, &name, sums.as_bytes(), signature.as_ref()),
            Ok([0xab; 32])
        );
        for altered in [
            format!("{sums}{sums}"),
            format!("{hash}  {name}.unrelated\n"),
        ] {
            assert_eq!(
                signed_checksum(
                    public,
                    &name,
                    altered.as_bytes(),
                    key.sign(altered.as_bytes()).as_ref()
                ),
                Err(Error::Verification)
            );
        }
        let mut tampered = sums.into_bytes();
        tampered[0] = b'c';
        assert_eq!(
            signed_checksum(public, &name, &tampered, signature.as_ref()),
            Err(Error::Verification)
        );
    }

    #[test]
    fn rejects_truncated_oversized_and_modified_downloads() {
        let data = b"expected";
        let digest = digest::digest(&digest::SHA256, data);
        let expected = digest.as_ref().try_into().unwrap();
        for bad in [&data[..3], b"modified", b"too long for the limit"] {
            assert_eq!(
                copy_verified(bad, io::sink(), Some(data.len() as u64), expected),
                Err(Error::Verification)
            );
        }
        assert_eq!(
            copy_verified(
                data.as_slice(),
                io::sink(),
                Some(data.len() as u64),
                expected
            ),
            Ok(())
        );
    }
}
