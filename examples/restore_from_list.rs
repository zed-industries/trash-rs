use chrono::{DateTime, Utc};
use trash::TrashItem;

#[cfg(not(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
)))]
fn main() {
    println!("This is currently only supported on Windows, Linux, and other Freedesktop.org compliant OSes");
}

struct ListEntry {
    removed_at: DateTime<Utc>,
    item: TrashItem,
}

impl std::fmt::Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.removed_at.to_rfc2822(), self.item.original_path().display()))
    }
}

#[cfg(any(
    target_os = "windows",
    all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))
))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trash_items: Vec<_> = trash::os_limited::list()?
        .into_iter()
        .filter_map(|item| Some(ListEntry { removed_at: DateTime::from_timestamp(item.time_deleted, 0)?, item }))
        .collect();

    if trash_items.is_empty() {
        eprintln!("Nothing in trash to show! exiting");
        return Ok(());
    }
    let to_restore = inquire::Select::new("Select an entry to restore from trash", trash_items).prompt()?;

    trash::restore_all([to_restore.item])?;
    Ok(())
}
