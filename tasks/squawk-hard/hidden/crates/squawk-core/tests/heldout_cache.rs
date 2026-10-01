//! `VocabCache`: one vocab per cwd, kept until the repo's index changes.

mod heldout_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use heldout_support::{git, git_out, git_repo, tempdir, write};
use squawk_core::context::VocabCache;

fn index_of(work_tree: &Path) -> PathBuf {
    let index = PathBuf::from(git_out(
        work_tree,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    ));
    assert!(index.is_file(), "index at {}", index.display());
    index
}

/// Date the index ten seconds back, so that the next `git add` moves its
/// mtime plainly, whatever resolution it is compared at, and without any
/// timestamp from the future.
fn age_index(work_tree: &Path) {
    let earlier = SystemTime::now() - Duration::from_secs(10);
    fs::File::options()
        .write(true)
        .open(index_of(work_tree))
        .unwrap()
        .set_modified(earlier)
        .unwrap();
}

fn stage_all(work_tree: &Path) {
    let index = index_of(work_tree);
    let before = fs::metadata(&index).unwrap().modified().unwrap();
    git(work_tree, &["add", "-A"]);
    let after = fs::metadata(&index).unwrap().modified().unwrap();
    assert!(after > before, "git add moved the index");
}

fn has(files: &[String], file: &str) -> bool {
    files.iter().any(|f| f == file)
}

#[test]
fn a_second_get_reuses_the_vocab() {
    let dir = git_repo(&[("src/audio.rs", "")]);
    let mut cache = VocabCache::new();
    let a = cache.get(&dir.path);
    assert!(has(&a.files, "src/audio.rs"), "{:?}", a.files);
    let b = cache.get(&dir.path);
    assert!(Arc::ptr_eq(&a, &b), "built again with nothing changed");
}

#[test]
fn each_cwd_has_its_own_vocab() {
    let one = git_repo(&[("src/audio.rs", "")]);
    let two = git_repo(&[("src/meeting.rs", "")]);
    let mut cache = VocabCache::new();
    let a = cache.get(&one.path);
    let b = cache.get(&two.path);
    assert!(has(&a.files, "src/audio.rs") && !has(&a.files, "src/meeting.rs"));
    assert!(has(&b.files, "src/meeting.rs") && !has(&b.files, "src/audio.rs"));
    assert!(Arc::ptr_eq(&a, &cache.get(&one.path)));
    assert!(Arc::ptr_eq(&b, &cache.get(&two.path)));
}

#[test]
fn a_changed_index_rebuilds_it() {
    let dir = git_repo(&[("src/audio.rs", "")]);
    age_index(&dir.path);
    let mut cache = VocabCache::new();
    let a = cache.get(&dir.path);
    assert!(!has(&a.files, "src/meeting.rs"));
    write(&dir.path, "src/meeting.rs", "");
    stage_all(&dir.path);
    let b = cache.get(&dir.path);
    assert!(has(&b.files, "src/meeting.rs"), "stale: {:?}", b.files);
    // And it settles again.
    assert!(Arc::ptr_eq(&b, &cache.get(&dir.path)));
}

#[test]
fn a_session_in_a_subdirectory_follows_the_repos_index() {
    let dir = git_repo(&[("app/src/main.rs", ""), ("lib/src/lib.rs", "")]);
    let app = dir.path.join("app");
    age_index(&dir.path);
    let mut cache = VocabCache::new();
    let a = cache.get(&app);
    assert!(has(&a.files, "src/main.rs"), "{:?}", a.files);
    assert!(Arc::ptr_eq(&a, &cache.get(&app)));
    write(&app, "src/meeting.rs", "");
    stage_all(&dir.path);
    let b = cache.get(&app);
    assert!(has(&b.files, "src/meeting.rs"), "stale: {:?}", b.files);
}

#[test]
fn a_linked_worktree_follows_its_own_index() {
    let dir = git_repo(&[("src/audio.rs", "")]);
    let tree = dir.path.parent().unwrap().join("widget-tree");
    git(
        &dir.path,
        &["worktree", "add", "-q", "-b", "tree", tree.to_str().unwrap()],
    );
    age_index(&tree);
    let mut cache = VocabCache::new();
    let a = cache.get(&tree);
    assert!(has(&a.files, "src/audio.rs"), "{:?}", a.files);
    assert!(Arc::ptr_eq(&a, &cache.get(&tree)));
    write(&tree, "src/meeting.rs", "");
    stage_all(&tree);
    let b = cache.get(&tree);
    assert!(has(&b.files, "src/meeting.rs"), "stale: {:?}", b.files);
}

#[test]
fn outside_git_a_second_get_reuses_the_vocab() {
    let dir = tempdir();
    write(&dir.path, "a.rs", "");
    let mut cache = VocabCache::new();
    let a = cache.get(&dir.path);
    assert!(has(&a.files, "a.rs"), "{:?}", a.files);
    assert!(Arc::ptr_eq(&a, &cache.get(&dir.path)));
}
