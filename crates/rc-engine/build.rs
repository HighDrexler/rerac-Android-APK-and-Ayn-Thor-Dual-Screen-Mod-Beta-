//! Bind the native binary to the actual Cargo version and reviewed upstream pin.
use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref()!=Ok("android") {return;}
    let pin=PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../android/native/upstream.lock");
    println!("cargo:rerun-if-changed={}",pin.display());
    for key in ["RERAC_SOURCE_REVISION","_CARGO_NDK_LINK_TARGET"] {println!("cargo:rerun-if-env-changed={key}");}
    let text=fs::read_to_string(&pin).expect("Android upstream.lock is required");
    let get=|key:&str| text.lines().filter_map(|l|l.split_once('=')).find(|(k,_)|k.trim()==key).map(|(_,v)|v.trim().to_owned()).expect("missing upstream pin field");
    let version=env::var("CARGO_PKG_VERSION").unwrap();
    assert_eq!(version,get("version"),"Android pin must match the actual core workspace version");
    let upstream=get("commit");
    let sha=|s:&str|s.len()==40&&s.bytes().all(|b|b.is_ascii_hexdigit());
    assert!(sha(&upstream),"full upstream SHA required");
    let source=env::var("RERAC_SOURCE_REVISION").unwrap_or_else(|_| {
        let out=Command::new("git").args(["rev-parse","HEAD"]).output().expect("source revision missing");
        assert!(out.status.success());String::from_utf8(out.stdout).unwrap().trim().to_owned()
    });
    assert!(sha(&source),"full source SHA required");
    let target=env::var("TARGET").unwrap();
    assert!(matches!(target.as_str(),"aarch64-linux-android"|"x86_64-linux-android"));
    // cargo-ndk 4.1.2 calls its ABI (e.g. x86_64) ANDROID_PLATFORM.
    // Its actual Clang linker target includes the selected numeric API level.
    let link=env::var("_CARGO_NDK_LINK_TARGET").expect("Build with pinned cargo-ndk");
    let triple=link.trim().strip_prefix("--target=").unwrap_or(link.trim());
    let sdk=triple.strip_prefix(target.as_str()).and_then(|n|n.parse::<u32>().ok())
        .unwrap_or_else(||panic!("Invalid Android linker target {link:?} for {target}"));
    assert_eq!(sdk,26,"Preserve the original Android 8.0 compatibility contract");
    assert!(version.bytes().all(|b|b.is_ascii_alphanumeric()||b".-+".contains(&b)));
    let json=format!(r#"{{"schema":1,"core_version":"{version}","upstream_sha":"{upstream}","source_sha":"{source}","target":"{target}","min_sdk":{sdk},"adapter_api":1}}"#);
    let marker=format!("RERAC_ANDROID_CORE_JSON:{json}\0");
    let offset = "RERAC_ANDROID_CORE_JSON:".len();
    let code=format!("#[used]\nstatic CORE_STAMP: &[u8] = b{marker:?};\n#[no_mangle]\npub extern \"C\" fn rerac_android_core_info_v1() -> *const std::ffi::c_char {{ CORE_STAMP.as_ptr().wrapping_add({offset}).cast() }}\n");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("android_core_build.rs"),code).unwrap();
}
