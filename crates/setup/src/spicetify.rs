//! Installing and running the Spicetify CLI.

use std::fmt::Write as _;
use std::io::{Cursor, Read};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::Paths;

const LATEST_RELEASE: &str = "https://api.github.com/repos/spicetify/cli/releases/latest";
const USER_AGENT: &str = concat!("focuslock/", env!("CARGO_PKG_VERSION"));
/// The Windows zips are a few MB; anything far bigger is not Spicetify.
const MAX_DOWNLOAD: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    /// `sha256:<hex>`, computed by GitHub when the asset was uploaded.
    pub digest: Option<String>,
}

/// The Windows zip for `arch` (as in `std::env::consts::ARCH`).
///
/// # Errors
/// If the architecture is unsupported or the release has no matching zip.
pub fn select_asset<'a>(release: &'a Release, arch: &str) -> anyhow::Result<&'a Asset> {
    let arch = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "x32",
        other => bail!("Spicetify has no Windows build for {other}"),
    };
    let suffix = format!("-windows-{arch}.zip");
    release
        .assets
        .iter()
        .find(|a| a.name.starts_with("spicetify-") && a.name.ends_with(&suffix))
        .with_context(|| format!("Spicetify {} has no {suffix} download", release.tag_name))
}

/// Checks `bytes` against a GitHub `sha256:<hex>` digest.
///
/// # Errors
/// If the digest is missing, malformed or doesn't match.
pub fn verify_digest(bytes: &[u8], digest: Option<&str>) -> anyhow::Result<()> {
    let expected = digest
        .and_then(|d| d.strip_prefix("sha256:"))
        .context("the download has no SHA-256 checksum to verify")?;
    let actual = Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
    if !actual.eq_ignore_ascii_case(expected) {
        bail!("the Spicetify download is corrupted (checksum mismatch)");
    }
    Ok(())
}

/// Downloads the latest Spicetify release, verifies it and extracts it into
/// `dir`, overwriting files with the same name.
///
/// # Errors
/// On network, checksum or extraction errors.
pub fn install_latest(dir: &Path) -> anyhow::Result<()> {
    let release: Release = ureq::get(LATEST_RELEASE)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("couldn't reach GitHub to download Spicetify")?
        .body_mut()
        .read_json()?;
    let asset = select_asset(&release, std::env::consts::ARCH)?;
    let bytes = ureq::get(&asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .call()
        .context("couldn't download Spicetify")?
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()?;
    verify_digest(&bytes, asset.digest.as_deref())?;
    extract(&bytes, dir)
}

/// Extracts a zip into `dir`, skipping entries that would escape it.
fn extract(zip: &[u8], dir: &Path) -> anyhow::Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).context("not a valid zip")?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let target = dir.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        std::fs::write(&target, contents)
            .with_context(|| format!("writing {}", target.display()))?;
    }
    Ok(())
}

/// Runs the Spicetify CLI without a console window.
pub(crate) fn run(paths: &Paths, args: &[&str]) -> anyhow::Result<()> {
    let mut command = Command::new(paths.spicetify_exe());
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .with_context(|| format!("couldn't run spicetify {}", args.join(" ")))?;
    if !output.status.success() {
        let text =
            String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
        bail!(
            "spicetify {} failed: {}",
            args.join(" "),
            last_lines(&strip_ansi(&text), 3)
        );
    }
    Ok(())
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip a CSI sequence: ESC [ ... final byte in @..~
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    lines[lines.len().saturating_sub(n)..].join(" ")
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn release() -> Release {
        let asset = |name: &str| Asset {
            name: name.into(),
            browser_download_url: format!("https://example.invalid/{name}"),
            digest: None,
        };
        Release {
            tag_name: "v2.45.3".into(),
            assets: vec![
                asset("spicetify-2.45.3-linux-amd64.tar.gz"),
                asset("spicetify-2.45.3-windows-arm64.zip"),
                asset("spicetify-2.45.3-windows-x64.zip"),
            ],
        }
    }

    #[test]
    fn selects_the_windows_zip() {
        let release = release();
        assert_eq!(
            select_asset(&release, "x86_64").unwrap().name,
            "spicetify-2.45.3-windows-x64.zip"
        );
        assert_eq!(
            select_asset(&release, "aarch64").unwrap().name,
            "spicetify-2.45.3-windows-arm64.zip"
        );
        assert!(select_asset(&release, "x86").is_err());
        assert!(select_asset(&release, "riscv64").is_err());
    }

    #[test]
    fn digests() {
        let hash = "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_digest(b"hello", Some(hash)).is_ok());
        assert!(verify_digest(b"hellO", Some(hash)).is_err());
        assert!(verify_digest(b"hello", None).is_err());
        assert!(verify_digest(b"hello", Some("md5:abc")).is_err());
    }

    #[test]
    fn extracts_and_skips_escaping_entries() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("spicetify.exe", "exe"),
            ("jsHelper/spicetifyWrapper.js", "js"),
            ("../evil.txt", "nope"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        let bytes = zip.finish().unwrap().into_inner();

        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("spicetify");
        extract(&bytes, &dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("spicetify.exe")).unwrap(),
            "exe"
        );
        assert!(dir.join("jsHelper/spicetifyWrapper.js").is_file());
        assert!(!root.path().join("evil.txt").exists());
    }

    #[test]
    fn cleans_cli_output() {
        let text = "\x1b[32mok\x1b[0m line\n\n\x1b[31merror\x1b[0m: Spotify is running\n";
        assert_eq!(
            last_lines(&strip_ansi(text), 3),
            "ok line error: Spotify is running"
        );
    }

    /// Hits GitHub: `cargo test -p focuslock-setup -- --ignored`.
    #[test]
    #[ignore = "needs network"]
    fn downloads_the_latest_release() {
        let dir = tempfile::tempdir().unwrap();
        install_latest(dir.path()).unwrap();
        assert!(dir.path().join("spicetify.exe").is_file());
    }
}
