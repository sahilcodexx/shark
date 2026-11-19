use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// Embedded icon assets for Shark.
pub struct Assets;

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[
            $(($name, include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*
        ];
    };
}

icons!(
    "add-01",
    "app-window",
    "archive",
    "arrow-down-circle",
    "arrow-up",
    "audio",
    "bookmark",
    "book",
    "bot",
    "cancel-01",
    "check-hi",
    "chevron-down-hi",
    "chevron-left",
    "chevron-right",
    "clock",
    "cloud",
    "code",
    "copy",
    "cpu",
    "database",
    "document",
    "download",
    "eject",
    "external-link",
    "eye",
    "file",
    "file-text",
    "filter",
    "folder-01",
    "folder",
    "git-branch",
    "git-commit",
    "globe",
    "grid",
    "hard-drive",
    "home",
    "image",
    "keyboard",
    "layers",
    "link",
    "list",
    "lock",
    "more-horizontal",
    "palette",
    "pause",
    "pdf",
    "pencil",
    "play",
    "rocket",
    "rotate-ccw",
    "rotate-cw",
    "search",
    "server",
    "settings",
    "share",
    "sidebar-left",
    "sliders",
    "table",
    "tag",
    "terminal",
    "trash",
    "user",
    "video",
    "volume",
);

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let name = path.trim_end_matches(".svg");
        Ok(ICONS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| (*name).into())
            .collect())
    }
}
