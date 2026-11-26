//! Read a type-2 AppImage icon without executing or mounting the image.
use backhand::{FilesystemReader, InnerNode};
use md5::{Digest, Md5};
use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::{ffi::OsStrExt, fs::OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

const ICON_LIMIT: u64 = 2 * 1024 * 1024;

fn squashfs_offset(header: &[u8; 64]) -> Option<u64> {
    if !header.starts_with(b"\x7fELF") || header[5] != 1 || &header[8..11] != b"AI\x02" {
        return None;
    }
    let (start, entry, count) = match header[4] {
        1 => (
            u32::from_le_bytes(header[32..36].try_into().ok()?) as u64,
            46,
            48,
        ),
        2 => (u64::from_le_bytes(header[40..48].try_into().ok()?), 58, 60),
        _ => return None,
    };
    let size = u16::from_le_bytes(header[entry..entry + 2].try_into().ok()?) as u64;
    let entries = u16::from_le_bytes(header[count..count + 2].try_into().ok()?) as u64;
    start
        .checked_add(size * entries)
        .filter(|_| start > 0 && size > 0 && entries > 0)
}

fn read_icon(mut file: File) -> Option<(Vec<u8>, &'static str)> {
    let mut header = [0; 64];
    file.read_exact(&mut header).ok()?;
    let offset = squashfs_offset(&header)?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut superblock = [0; 8];
    file.read_exact(&mut superblock).ok()?;
    // Cap metadata work when browsing arbitrary downloaded executables.
    if &superblock[..4] != b"hsqs" || u32::from_le_bytes(superblock[4..].try_into().ok()?) > 100_000
    {
        return None;
    }
    let filesystem =
        FilesystemReader::from_reader_with_offset(BufReader::new(file), offset).ok()?;
    let mut icon = PathBuf::from("/.DirIcon");
    for _ in 0..8 {
        let node = filesystem.files().find(|node| node.fullpath == icon)?;
        match &node.inner {
            InnerNode::File(file) => {
                let mut bytes = Vec::new();
                filesystem
                    .file(file)
                    .reader_checked()
                    .ok()?
                    .take(ICON_LIMIT + 1)
                    .read_to_end(&mut bytes)
                    .ok()?;
                if bytes.len() as u64 > ICON_LIMIT {
                    return None;
                }
                let extension = if image::guess_format(&bytes).is_ok() {
                    "png"
                } else if bytes.windows(4).any(|part| part == b"<svg") {
                    "svg"
                } else {
                    return None;
                };
                return Some((bytes, extension));
            }
            InnerNode::Symlink(link) => {
                let mut target = icon.parent()?.to_path_buf();
                for component in link.link.components() {
                    match component {
                        Component::RootDir => target = "/".into(),
                        Component::ParentDir => {
                            target.pop();
                        }
                        Component::Normal(name) => target.push(name),
                        _ => {}
                    }
                }
                icon = target;
            }
            _ => return None,
        }
    }
    None
}

pub(crate) fn thumbnail(path: &Path) -> Option<PathBuf> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }
    let mut key = Md5::new();
    key.update(path.as_os_str().as_bytes());
    key.update(metadata.len().to_le_bytes());
    key.update(
        metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos()
            .to_le_bytes(),
    );
    let name = format!("{:x}", key.finalize());
    let dir = dirs::cache_dir()?.join("shark/appimage-icons");
    for ext in ["png", "svg"] {
        let existing = dir.join(&name).with_extension(ext);
        if existing.is_file() {
            return Some(existing);
        }
    }
    let (bytes, ext) = read_icon(file)?;
    fs::create_dir_all(&dir).ok()?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).ok()?;
    if ext == "svg" {
        temp.write_all(&bytes).ok()?;
    } else {
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .ok()?
            .thumbnail(256, 256)
            .write_to(&mut temp, image::ImageFormat::Png)
            .ok()?;
    }
    let target = dir.join(name).with_extension(ext);
    temp.persist(&target).ok()?;
    Some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_icon_through_archive_symlink() {
        let mut fs = backhand::FilesystemWriter::default();
        fs.push_file(Cursor::new(b"<svg/>"), "/logo.svg", Default::default())
            .unwrap();
        fs.push_symlink("/logo.svg", "/.DirIcon", Default::default())
            .unwrap();
        let mut file = tempfile::tempfile().unwrap();
        let mut header = [0; 64];
        header[..6].copy_from_slice(b"\x7fELF\x02\x01");
        header[8..11].copy_from_slice(b"AI\x02");
        header[40..48].copy_from_slice(&64u64.to_le_bytes());
        header[58..60].copy_from_slice(&64u16.to_le_bytes());
        header[60..62].copy_from_slice(&1u16.to_le_bytes());
        file.write_all(&header).unwrap();
        file.write_all(&[0; 64]).unwrap();
        let mut archive = Cursor::new(Vec::new());
        fs.write(&mut archive).unwrap();
        file.write_all(archive.get_ref()).unwrap();
        file.rewind().unwrap();
        assert_eq!(read_icon(file).unwrap(), (b"<svg/>".to_vec(), "svg"));
    }
}
