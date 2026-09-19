use std::path::PathBuf;

/// Locate the sibling CLI binary at runtime instead of retaining Cargo's
/// compile-time path. A copied or relocated target directory can otherwise
/// leave `CARGO_BIN_EXE_uni` pointing at the old checkout.
pub fn bin() -> PathBuf {
    let deps = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("test executable must have a parent")
        .to_path_buf();
    let binary = deps
        .parent()
        .expect("test executable must be in target/*/deps")
        .join(format!("uni{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.is_file(),
        "CLI binary missing at {}",
        binary.display()
    );
    binary
}

/// Find the workspace from the test process rather than its compilation path.
#[allow(dead_code)]
pub fn workspace_root() -> PathBuf {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(root) = executable
            .parent()
            .and_then(|deps| deps.parent())
            .and_then(|profile| profile.parent())
            .and_then(|target| target.parent())
        {
            if root.join("schemas/uni.schema.json").is_file() {
                return root.to_path_buf();
            }
        }
    }
    let mut dir = std::env::current_dir().expect("test working directory");
    loop {
        if dir.join("schemas/uni.schema.json").is_file() {
            return dir;
        }
        if !dir.pop() {
            panic!("could not find the UNI workspace from the test directory");
        }
    }
}
