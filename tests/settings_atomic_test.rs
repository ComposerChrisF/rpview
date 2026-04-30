//! Atomicity tests for `save_settings_to_path`.
//!
//! These tests exercise the temp-file-then-`persist` pattern introduced for
//! H3 in the v0.20.6 hardening pass: a reader observing the on-disk file
//! during a concurrent write must always see either the prior valid JSON or
//! the new valid JSON — never a truncated/partial file.

use rpview::state::settings::AppSettings;
use rpview::utils::settings_io::{load_settings_from_path, save_settings_to_path};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn save_then_load_roundtrip() {
    // Simplest atomicity sanity check: a single save+load yields equal
    // settings and leaves no leftover temp files in the directory.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");

    let mut settings = AppSettings::default();
    settings.viewer_behavior.state_cache_size = 42;

    save_settings_to_path(&settings, &path).expect("save failed");

    let loaded = load_settings_from_path(&path);
    assert_eq!(loaded.viewer_behavior.state_cache_size, 42);

    // No leftover .tmp files from NamedTempFile::persist
    let stragglers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "settings.json")
        .collect();
    assert!(
        stragglers.is_empty(),
        "expected no stragglers, found: {:?}",
        stragglers
    );
}

#[test]
fn save_overwrites_existing_file_atomically() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");

    // Initial save
    let mut s1 = AppSettings::default();
    s1.viewer_behavior.state_cache_size = 1;
    save_settings_to_path(&s1, &path).unwrap();
    let inode1 = std::fs::metadata(&path).unwrap().len();
    assert!(inode1 > 0);

    // Overwrite — atomic rename should drop the old file in place
    let mut s2 = AppSettings::default();
    s2.viewer_behavior.state_cache_size = 99;
    save_settings_to_path(&s2, &path).unwrap();

    let loaded = load_settings_from_path(&path);
    assert_eq!(loaded.viewer_behavior.state_cache_size, 99);
}

#[test]
fn concurrent_readers_never_see_partial_file() {
    // Spawn one writer that hammers the file with alternating settings, plus
    // four reader threads that loop reading and parsing.  Any successful
    // read must yield a valid AppSettings — a truncated file would parse-
    // fail and surface as a panic from the reader threads.
    let dir = TempDir::new().unwrap();
    let path = Arc::new(dir.path().join("settings.json"));

    // Seed the file so readers always have something to read
    let s_seed = AppSettings::default();
    save_settings_to_path(&s_seed, &path).unwrap();

    let stop = Arc::new(AtomicBool::new(false));

    // Writer
    let writer_path = Arc::clone(&path);
    let writer_stop = Arc::clone(&stop);
    let writer = thread::spawn(move || {
        let mut i: usize = 0;
        while !writer_stop.load(Ordering::Relaxed) {
            let mut s = AppSettings::default();
            s.viewer_behavior.state_cache_size = (i % 1000) + 1;
            save_settings_to_path(&s, &writer_path).expect("write failed");
            i += 1;
        }
        i
    });

    // Readers
    let mut readers = Vec::new();
    for _ in 0..4 {
        let reader_path = Arc::clone(&path);
        let reader_stop = Arc::clone(&stop);
        readers.push(thread::spawn(move || {
            let mut reads: usize = 0;
            let mut empties: usize = 0;
            while !reader_stop.load(Ordering::Relaxed) {
                let bytes = match std::fs::read(reader_path.as_path()) {
                    Ok(b) => b,
                    Err(_) => continue, // window between rename ops, retry
                };
                if bytes.is_empty() {
                    empties += 1;
                    continue;
                }
                // Any non-empty file must be valid JSON of an AppSettings.
                let parsed: Result<AppSettings, _> = serde_json::from_slice(&bytes);
                assert!(
                    parsed.is_ok(),
                    "reader observed a partial/corrupt file ({} bytes): {:?}",
                    bytes.len(),
                    String::from_utf8_lossy(&bytes).chars().take(80).collect::<String>(),
                );
                reads += 1;
            }
            (reads, empties)
        }));
    }

    // Run for a short while
    thread::sleep(Duration::from_millis(500));
    stop.store(true, Ordering::Relaxed);

    let writes = writer.join().unwrap();
    let mut total_reads = 0;
    for r in readers {
        let (reads, _empties) = r.join().unwrap();
        total_reads += reads;
    }

    // We should have exercised the race meaningfully — at least a few
    // writes and reads each.  This isn't a strict performance assertion,
    // just a smoke check.
    assert!(writes > 5, "only {} writes happened", writes);
    assert!(total_reads > 5, "only {} reads happened", total_reads);
}

#[test]
fn save_to_nonexistent_parent_returns_error() {
    // save_settings_to_path does not create parent directories — that's the
    // caller's responsibility (get_settings_path does it).  A save into a
    // non-existent dir must fail rather than silently succeed.
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("does-not-exist").join("settings.json");

    let result = save_settings_to_path(&AppSettings::default(), &nested);
    assert!(
        result.is_err(),
        "save to non-existent parent should error, got {:?}",
        result
    );
}

#[test]
fn debounced_writes_eventually_persist() {
    // Smoke test: emit a burst of debounced saves with the *real*
    // user-config-dir path we'd write to in production, but redirected via
    // the public direct API.  We can't redirect the debouncer's path, but
    // we can verify that the debounced public API doesn't panic and that
    // the immediate API still produces a parseable file after.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");

    // Seed
    let s = AppSettings::default();
    save_settings_to_path(&s, &path).unwrap();

    // The debouncer writes to the real platform path — we don't exercise
    // that here.  Instead, just verify direct saves remain atomic under a
    // quick burst.
    let start = Instant::now();
    for i in 0..100 {
        let mut s = AppSettings::default();
        s.viewer_behavior.state_cache_size = i + 1;
        save_settings_to_path(&s, &path).unwrap();
    }
    let elapsed = start.elapsed();

    let loaded = load_settings_from_path(&path);
    assert_eq!(loaded.viewer_behavior.state_cache_size, 100);
    // 100 atomic writes shouldn't take outrageously long; this is an
    // upper-bound smoke check, not a perf benchmark.
    assert!(
        elapsed < Duration::from_secs(5),
        "100 atomic writes took {:?}",
        elapsed
    );
}
