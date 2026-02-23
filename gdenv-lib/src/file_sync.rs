use anyhow::Context;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileEntry {
    rel_path: PathBuf,
    hash: u64,
    is_dir: bool,
}

pub fn sync_recursive(
    source_base: &Path,
    dest_base: &Path,
    includes: Option<&[PathBuf]>,
    excludes: Option<&[PathBuf]>,
) -> anyhow::Result<()> {
    let source_list = get_file_list(source_base).context("Failed to get source file list")?;
    let filtered_source_list = {
        let mut l = filter_file_list(source_list, includes, excludes);
        // Sort so parents are added before children
        l.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        l
    };

    let dest_list = get_file_list(dest_base).context("Failed to get destination file list")?;
    let filtered_dest_list = {
        let mut l = filter_file_list(dest_list, includes, None);
        // Reverse sort so children are removed before parents
        l.sort_by(|a, b| b.rel_path.cmp(&a.rel_path));
        l
    };

    // For every file in the destination list that isn't in the source list, delete it.
    // If it is in the source list, but the hash doesn't match, delete it.
    for dest_entry in &filtered_dest_list {
        let target_path = dest_base.join(&dest_entry.rel_path);
        let is_matching = filtered_source_list.iter().any(|s| s == dest_entry);

        if !is_matching {
            if dest_entry.is_dir {
                tracing::debug!("Removing directory {:?}", target_path);
                fs::remove_dir_all(&target_path).context(format!(
                    "Failed to remove target directory: {:?}",
                    target_path
                ))?;
            } else {
                tracing::debug!("Removing file {:?}", target_path);
                fs::remove_file(&target_path)
                    .context(format!("Failed to remove target file: {:?}", target_path))?;
            }
        }
    }

    // For every file in the source list that isn't in the destination list, copy it over.
    for source_entry in &filtered_source_list {
        let source_path = source_base.join(&source_entry.rel_path);
        let target_path = dest_base.join(&source_entry.rel_path);

        if source_entry.is_dir {
            tracing::debug!("Creating directory {:?}", target_path);
            fs::create_dir_all(&target_path).context(format!(
                "Failed to create target directory: {:?}",
                target_path
            ))?;
        } else {
            tracing::debug!("Copying file {:?} to {:?}", source_path, target_path);
            fs::copy(&source_path, &target_path).with_context(|| {
                format!("Failed to copy {:?} to {:?}", source_path, target_path)
            })?;
        }
    }

    Ok(())
}

fn get_file_list(base: &Path) -> anyhow::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    if !base.exists() {
        return Ok(entries);
    }
    for entry in WalkDir::new(base).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel_path = path.strip_prefix(base).context("Failed to strip prefix")?;

        if rel_path.as_os_str().is_empty() {
            continue;
        }

        let is_dir = path.is_dir();
        let hash = if is_dir {
            0
        } else {
            let contents = fs::read(path)?;
            let mut hasher = DefaultHasher::new();
            contents.hash(&mut hasher);
            hasher.finish()
        };

        entries.push(FileEntry {
            rel_path: rel_path.to_path_buf(),
            hash,
            is_dir,
        });
    }
    Ok(entries)
}

fn filter_file_list(
    list: Vec<FileEntry>,
    includes: Option<&[PathBuf]>,
    excludes: Option<&[PathBuf]>,
) -> Vec<FileEntry> {
    list.into_iter()
        .filter(|entry| {
            let rel_path = &entry.rel_path;

            // Skip if it matches any exclude pattern
            if excludes.is_some_and(|exs| exs.iter().any(|ex| rel_path.starts_with(ex))) {
                return false;
            }

            // If includes are specified, the path must be inside one of them
            // or be a parent of one of them (to allow traversing into it)
            includes.is_none_or(|incs| {
                incs.iter()
                    .any(|inc| rel_path.starts_with(inc) || inc.starts_with(rel_path))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempdir::TempDir;

    #[test]
    fn test_filter_file_list() {
        let list = vec![
            FileEntry {
                rel_path: PathBuf::from("src/main.rs"),
                hash: 1,
                is_dir: false,
            },
            FileEntry {
                rel_path: PathBuf::from("src/lib.rs"),
                hash: 2,
                is_dir: false,
            },
            FileEntry {
                rel_path: PathBuf::from("docs/index.html"),
                hash: 3,
                is_dir: false,
            },
            FileEntry {
                rel_path: PathBuf::from("target/debug/app"),
                hash: 4,
                is_dir: false,
            },
        ];

        // Test 1: No filters
        let filtered = filter_file_list(list.clone(), None, None);
        assert_eq!(filtered.len(), 4);

        // Test 2: Only excludes
        let excludes = vec![PathBuf::from("target"), PathBuf::from("docs/index.html")];
        let filtered = filter_file_list(list.clone(), None, Some(&excludes));
        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .any(|e| e.rel_path == PathBuf::from("src/main.rs"))
        );
        assert!(
            filtered
                .iter()
                .any(|e| e.rel_path == PathBuf::from("src/lib.rs"))
        );

        // Test 3: Only includes
        let includes = vec![PathBuf::from("src")];
        let filtered = filter_file_list(list.clone(), Some(&includes), None);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.rel_path.starts_with("src")));

        // Test 4: Combined includes and excludes
        // Include everything in 'src', but exclude 'src/lib.rs'
        let includes = vec![PathBuf::from("src")];
        let excludes = vec![PathBuf::from("src/lib.rs")];
        let filtered = filter_file_list(list.clone(), Some(&includes), Some(&excludes));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rel_path, PathBuf::from("src/main.rs"));

        // Test 5: Include a parent directory of a file in the list
        let includes = vec![PathBuf::from("docs")];
        let filtered = filter_file_list(list.clone(), Some(&includes), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rel_path, PathBuf::from("docs/index.html"));
    }

    #[test]
    fn test_get_file_list() -> anyhow::Result<()> {
        let tmp_dir = TempDir::new("get_file_list_test")?;
        let base = tmp_dir.path();

        // Create a structure:
        // /file1.txt
        // /file1_copy.txt (same content as file1)
        // /dir1/
        // /dir1/file2.txt (different content)
        let file1_path = base.join("file1.txt");
        let file1_copy_path = base.join("file1_copy.txt");
        let dir1_path = base.join("dir1");
        let file2_path = dir1_path.join("file2.txt");

        fs::create_dir(&dir1_path)?;
        fs::write(&file1_path, "content1")?;
        fs::write(&file1_copy_path, "content1")?;
        fs::write(&file2_path, "content2")?;

        let list = get_file_list(base)?;

        // Expecting 4 entries: file1.txt, file1_copy.txt, dir1, dir1/file2.txt
        assert_eq!(list.len(), 4);

        let f1 = list
            .iter()
            .find(|e| e.rel_path == PathBuf::from("file1.txt"))
            .unwrap();
        let f1_copy = list
            .iter()
            .find(|e| e.rel_path == PathBuf::from("file1_copy.txt"))
            .unwrap();
        let f2 = list
            .iter()
            .find(|e| e.rel_path == PathBuf::from("dir1/file2.txt"))
            .unwrap();
        let d1 = list
            .iter()
            .find(|e| e.rel_path == PathBuf::from("dir1"))
            .unwrap();

        // Verify hashes for identical files are equal
        assert_eq!(
            f1.hash, f1_copy.hash,
            "Hashes of files with identical content must match"
        );
        assert_ne!(f1.hash, 0);

        // Verify hashes for different files are different
        assert_ne!(
            f1.hash, f2.hash,
            "Hashes of files with different content must not match"
        );

        // Verify directory properties
        assert!(d1.is_dir);
        assert_eq!(d1.hash, 0, "Directories must have a hash of 0");

        Ok(())
    }

    #[test]
    fn test_sync_recursive() -> anyhow::Result<()> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .try_init();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let test_addon1_path: PathBuf = [manifest_dir, "test-data", "test-addon1-repo"]
            .iter()
            .collect();
        let test_addon1v2_path: PathBuf = [manifest_dir, "test-data", "test-addon1v2-repo"]
            .iter()
            .collect();
        let tmp_dir = TempDir::new("gdenv-test")?;

        tracing::info!(
            "Syncing {} to {}",
            test_addon1_path.display(),
            tmp_dir.path().display()
        );
        sync_recursive(&test_addon1_path, &tmp_dir.path(), None, None)?;

        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/plugin.cfg")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/changed_file.txt")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/old_file.txt")
                .exists()
        );
        assert!(
            !tmp_dir
                .path()
                .join("addons/test-addon1/new_file.txt")
                .exists()
        );
        assert!(tmp_dir.path().join("file-not-part-of-addon.txt").exists());
        let old_contents =
            fs::read_to_string(tmp_dir.path().join("addons/test-addon1/changed_file.txt"))?;

        tracing::info!(
            "Syncing {} to {}",
            test_addon1v2_path.display(),
            tmp_dir.path().display()
        );
        sync_recursive(&test_addon1v2_path, &tmp_dir.path(), None, None)?;

        let new_contents =
            fs::read_to_string(tmp_dir.path().join("addons/test-addon1/changed_file.txt"))?;
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/plugin.cfg")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/changed_file.txt")
                .exists()
        );
        assert!(
            !tmp_dir
                .path()
                .join("addons/test-addon1/old_file.txt")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("addons/test-addon1/new_file.txt")
                .exists()
        );
        assert!(tmp_dir.path().join("file-not-part-of-addon.txt").exists());
        assert_ne!(new_contents, old_contents);

        Ok(())
    }
}
