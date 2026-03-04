#[cfg(test)]
use std::path::PathBuf;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
pub(crate) fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
pub(crate) fn capture_original_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

#[cfg(test)]
pub(crate) fn restore_original_dir(original_dir: PathBuf) {
    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = if original_dir.exists() {
        original_dir
    } else {
        fallback
    };
    std::env::set_current_dir(target).expect("failed to restore working directory");
}
