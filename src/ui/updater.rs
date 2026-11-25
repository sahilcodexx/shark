use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

pub(super) const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = "sahilcodexx/shark";
const ASSET: &str = "Shark-x86_64.AppImage";

#[derive(Default)]
pub(super) enum Update {
    #[default]
    Idle,
    Checking,
    Downloading(u64, u64),
    Installing,
    Current,
    Installed(Version),
    Failed(String),
}

impl Update {
    pub(super) fn busy(&self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading(..) | Self::Installing
        )
    }

    pub(super) fn status(&self) -> String {
        match self {
            Self::Idle => String::new(),
            Self::Checking => "Checking for updates…".into(),
            Self::Downloading(bytes, total) => format!(
                "Downloading {}% · {:.2} / {:.2} MiB",
                bytes * 100 / total,
                *bytes as f64 / 1048576.,
                *total as f64 / 1048576.
            ),
            Self::Installing => "Installing update…".into(),
            Self::Current => "Shark is up to date".into(),
            Self::Installed(version) => format!("v{version} installed. Reopen Shark to finish."),
            Self::Failed(error) => error.clone(),
        }
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    digest: Option<String>,
}

pub(super) fn available() -> bool {
    cfg!(all(target_os = "linux", target_arch = "x86_64"))
        && std::env::var_os("APPIMAGE").is_some_and(|value| PathBuf::from(value).is_absolute())
}

fn image_header(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x7fELF") && bytes.get(8..11) == Some(b"AI\x02")
}

fn fetch<T>(url: &str, consume: impl FnOnce(&mut dyn Read) -> io::Result<T>) -> io::Result<T> {
    let mut curl = Command::new("curl")
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "300",
            "--user-agent",
            "Shark-AppImage-Updater",
            "--",
            url,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("Could not run curl: {error}")))?;
    let result = consume(&mut curl.stdout.take().expect("curl stdout is piped"));
    if result.is_err() {
        let _ = curl.kill();
    }
    let status = curl.wait()?;
    result.and_then(|value| {
        if status.success() {
            Ok(value)
        } else {
            Err(io::Error::other(format!("Download failed ({status})")))
        }
    })
}

fn download(
    input: &mut dyn Read,
    output: &mut File,
    size: u64,
    digest: &str,
    mut progress: impl FnMut(Update),
) -> io::Result<()> {
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut header = [0; 11];
    input.read_exact(&mut header)?;
    if !image_header(&header) {
        return Err(io::Error::other("The update is not a Linux AppImage"));
    }
    hash.update(header);
    output.write_all(&header)?;
    bytes += header.len() as u64;
    let mut last_update = Instant::now();
    let mut buffer = [0; 65536];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        if bytes > size {
            return Err(io::Error::other("Update exceeds the published size"));
        }
        hash.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
        if last_update.elapsed() >= Duration::from_millis(100) {
            progress(Update::Downloading(bytes, size));
            last_update = Instant::now();
        }
    }
    if bytes != size || format!("{:x}", hash.finalize()) != digest {
        return Err(io::Error::other("Update failed integrity verification"));
    }
    progress(Update::Downloading(size, size));
    Ok(())
}

pub(super) fn run(mut progress: impl FnMut(Update)) -> Result<Update, Box<dyn std::error::Error>> {
    if !available() {
        return Err("Automatic updates require the Linux x86-64 AppImage".into());
    }
    let image = PathBuf::from(std::env::var_os("APPIMAGE").ok_or("AppImage path unavailable")?)
        .canonicalize()?;
    let mut current = File::open(&image)?;
    let metadata = current.metadata()?;
    let mut header = [0; 11];
    current.read_exact(&mut header)?;
    if !metadata.is_file() || !image_header(&header) {
        return Err("The current AppImage could not be identified".into());
    }

    let release = fetch(
        &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        |input| {
            let mut json = Vec::new();
            input.take(1024 * 1024).read_to_end(&mut json)?;
            Ok(json)
        },
    )?;
    let release: Release = serde_json::from_slice(&release)?;
    let version = Version::parse(release.tag_name.trim_start_matches('v'))?;
    if !version.pre.is_empty() || version <= Version::parse(VERSION)? {
        return Ok(Update::Current);
    }
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == ASSET)
        .ok_or("The Linux AppImage for this release is not available yet")?;
    let digest = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("GitHub did not provide an AppImage checksum")?;
    if !(11..=512 * 1024 * 1024).contains(&asset.size) {
        return Err("The published AppImage size is outside the supported range".into());
    }

    // A sibling temporary file lets rename replace the installed image atomically.
    let mut temp = NamedTempFile::new_in(image.parent().ok_or("AppImage directory unavailable")?)?;
    progress(Update::Downloading(0, asset.size));
    fetch(
        &format!("https://github.com/{REPO}/releases/download/v{version}/{ASSET}"),
        |input| download(input, temp.as_file_mut(), asset.size, digest, &mut progress),
    )?;
    progress(Update::Installing);
    current.try_lock()?;
    let installed = fs::symlink_metadata(&image)?;
    if installed.ino() != metadata.ino() || installed.dev() != metadata.dev() {
        return Err("The AppImage changed during the download. Reopen Shark.".into());
    }
    temp.as_file()
        .set_permissions(fs::Permissions::from_mode(metadata.mode() & 0o777))?;
    temp.as_file().sync_all()?;
    temp.persist(&image)?;
    Ok(Update::Installed(version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_checks_size_hash_and_header() {
        let image = b"\x7fELF\0\0\0\0AI\x02payload";
        let digest = format!("{:x}", Sha256::digest(image));
        let mut file = tempfile::tempfile().unwrap();
        let size = image.len() as u64;
        let mut completed = false;
        download(&mut &image[..], &mut file, size, &digest, |state| {
            completed = matches!(state, Update::Downloading(done, total) if done == total)
        })
        .unwrap();
        assert!(completed);
        assert!(download(&mut &image[..], &mut file, size, "bad", |_| {}).is_err());
        assert!(download(&mut &image[..], &mut file, size + 1, &digest, |_| {}).is_err());
        assert!(
            download(
                &mut &b"not an AppImage"[..],
                &mut file,
                size,
                &digest,
                |_| {}
            )
            .is_err()
        );
    }
}
