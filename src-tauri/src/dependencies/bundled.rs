use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
/// Bundled executables are checked before use, including every transport helper.
pub fn verified_binary(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    let candidates = if let Some(dir)=std::env::var_os("AETHER_DESKTOP_RUNTIME_DIR") { vec![PathBuf::from(dir)] } else { vec![parent.join("../Resources/runtime"), parent.join("runtime"), Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/runtime")] };
    for dir in candidates {
        let result = (|| {
            let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).ok()?).ok()?;
            for (rel, hash) in manifest["files"].as_object()? {
                let p = Path::new(rel);
                if p.is_absolute() || p.components().any(|c| matches!(c,std::path::Component::ParentDir)) { return None; }
                let actual = format!("{:x}", Sha256::digest(std::fs::read(dir.join(p)).ok()?));
                if Some(actual.as_str()) != hash.as_str() { return None; }
            }
            let filename = if cfg!(windows) { format!("{}.exe", name) } else { name.to_string() };
            let path = dir.join(&filename);
            manifest["files"].get(&filename)?;
            path.exists().then_some(path)
        })();
        if result.is_some() { return result; }
    }
    None
}
