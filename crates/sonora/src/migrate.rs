use std::fs;
use std::io;
use std::path::Path;

const OLD: &str = "sonora";
const NEW: &str = "r3tro-core";

pub fn from_sonora() {
    let roots = [
        dirs::config_dir(),
        dirs::data_local_dir(),
        dirs::cache_dir(),
        dirs::state_dir(),
    ];
    let mut done = Vec::new();
    for root in roots.into_iter().flatten() {
        if done.contains(&root) {
            continue;
        }
        done.push(root.clone());
        let (old, new) = (root.join(OLD), root.join(NEW));
        if new.exists() || !old.is_dir() {
            continue;
        }
        if let Err(error) = copy(&old, &new) {
            eprintln!("r3tro-core: cannot import {}: {error}", old.display());
            fs::remove_dir_all(&new).ok();
        }
    }
}

fn copy(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        match entry.file_type()?.is_dir() {
            true => copy(&entry.path(), &target)?,
            false => {
                fs::copy(entry.path(), &target)?;
            }
        }
    }
    Ok(())
}
