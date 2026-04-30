//! Persistent on-disk frame cache for animated images.
//!
//! Animated GIF / WEBP frames — both raw decoded frames and local-contrast
//! processed outputs — are cached to `dirs::cache_dir()/rpview/cache/` so
//! repeat opens (and re-applies of the same LC parameters) skip the decode
//! and the LC computation.
//!
//! # Filename layout
//!
//! ```text
//! {image_key}_raw_{frame:06}.png             — unprocessed decoded frame
//! {image_key}_lc{params_hash}_{frame:06}.png — LC-processed at given params
//! ```
//!
//! Where:
//! - `image_key` = `{path_fnv:016x}_{mtime_secs}` — derived from the canonical
//!   source path plus its modification time, so replacing the file on disk
//!   invalidates the cache automatically.
//! - `params_hash` = 8 lowercase hex digits — low 32 bits of FNV-1a of a
//!   canonical serde_json serialization of [`local_contrast::Parameters`].
//!
//! FNV-1a is used (not `std::hash::DefaultHasher`) because the std hasher is
//! explicitly not guaranteed stable across compiler versions, and we need
//! cache keys to round-trip across app updates.
//!
//! TODO(cache size): warn the user when `total_size()` exceeds some
//! threshold (~5 GB?). With per-param-set caching of 100+ frame animations,
//! disk usage can climb fast and there is currently no automatic eviction.

use std::path::{Path, PathBuf};

use crate::utils::local_contrast::Parameters;

const CACHE_SUBDIR: &str = "rpview/cache";

/// FNV-1a 64-bit hash. Stable across compiler versions, unlike
/// `std::hash::DefaultHasher` (SipHash).
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Returns the cache root directory, creating it if necessary.
pub fn cache_root() -> Result<PathBuf, String> {
    let base = dirs::cache_dir().ok_or_else(|| "no platform cache dir".to_string())?;
    let dir = base.join(CACHE_SUBDIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {}", dir.display(), e))?;
    Ok(dir)
}

/// Returns a stable identifier for an image file: `{path_fnv:016x}_{mtime}`.
///
/// `None` if the path cannot be canonicalized or the mtime cannot be read —
/// callers should treat such images as uncacheable.
pub fn image_key(path: &Path) -> Option<String> {
    let canonical = path.canonicalize().ok()?;
    let metadata = std::fs::metadata(&canonical).ok()?;
    let mtime_secs = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let path_hash = fnv1a_64(canonical.as_os_str().as_encoded_bytes());
    Some(format!("{path_hash:016x}_{mtime_secs}"))
}

/// Returns 8 lowercase hex digits identifying an LC parameter set.
pub fn params_hash(params: &Parameters) -> String {
    let canonical = serde_json::to_string(params).unwrap_or_default();
    let h = fnv1a_64(canonical.as_bytes()) as u32;
    format!("{h:08x}")
}

/// Path for an unprocessed (raw) cached frame.
pub fn raw_frame_path(image_key: &str, frame_idx: usize) -> Result<PathBuf, String> {
    Ok(cache_root()?.join(format!("{image_key}_raw_{frame_idx:06}.png")))
}

/// Path for an LC-processed cached frame.
pub fn lc_frame_path(
    image_key: &str,
    params_hash: &str,
    frame_idx: usize,
) -> Result<PathBuf, String> {
    Ok(cache_root()?.join(format!("{image_key}_lc{params_hash}_{frame_idx:06}.png")))
}

/// Delete every cache file whose name starts with `image_key`. Returns the
/// number of bytes freed.
pub fn purge_image(image_key: &str) -> Result<u64, String> {
    let root = cache_root()?;
    let mut freed = 0u64;
    for entry in
        std::fs::read_dir(&root).map_err(|e| format!("read {}: {}", root.display(), e))?
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if !name_str.starts_with(image_key) {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if std::fs::remove_file(entry.path()).is_ok() {
            freed += size;
        }
    }
    Ok(freed)
}

/// Delete every file under the cache directory. Returns the number of bytes
/// freed. The directory itself is recreated (empty) on success.
pub fn purge_all() -> Result<u64, String> {
    let root = cache_root()?;
    let freed = total_size(&root).unwrap_or(0);
    std::fs::remove_dir_all(&root)
        .map_err(|e| format!("remove {}: {}", root.display(), e))?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("create {}: {}", root.display(), e))?;
    Ok(freed)
}

/// Total size in bytes of every regular file directly under `dir`.
pub fn total_size(dir: &Path) -> Result<u64, String> {
    let mut total = 0u64;
    let read = std::fs::read_dir(dir).map_err(|e| format!("read {}: {}", dir.display(), e))?;
    for entry in read.flatten() {
        if let Ok(metadata) = entry.metadata()
            && metadata.is_file()
        {
            total += metadata.len();
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_known_vectors() {
        // FNV-1a 64-bit reference vectors (offset basis & first iteration).
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        // Differing inputs produce differing hashes; same input is stable.
        assert_ne!(fnv1a_64(b"abc"), fnv1a_64(b"abd"));
        assert_eq!(fnv1a_64(b"hello world"), fnv1a_64(b"hello world"));
    }

    #[test]
    fn params_hash_is_deterministic() {
        let p = Parameters::default();
        assert_eq!(params_hash(&p), params_hash(&p));
        assert_eq!(params_hash(&p).len(), 8);
    }

    #[test]
    fn params_hash_differs_when_params_differ() {
        let mut a = Parameters::default();
        let mut b = Parameters::default();
        b.contrast += 0.1;
        assert_ne!(params_hash(&a), params_hash(&b));
        b = a.clone();
        a.lighten_shadows = 0.1;
        assert_ne!(params_hash(&a), params_hash(&b));
    }

    #[test]
    fn image_key_returns_none_for_missing_file() {
        let missing = std::env::temp_dir().join("rpview_does_not_exist_xyzzy.bin");
        assert!(image_key(&missing).is_none());
    }
}
