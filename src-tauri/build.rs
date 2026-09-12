fn main() {
    // Windows：统一由链接器 /MANIFEST:EMBED 嵌入 app.manifest（bins 与 test 二进制都需要；
    // tauri-build 的 resource.lib 只覆盖 bins，cargo test 的 lib 测试 exe 缺清单会因
    // comctl32 5.82 缺 v6 导出而启动失败），因此关闭 tauri-build 默认清单避免资源重复（CVT1100）。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("app.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
    )
    .expect("failed to run tauri-build");
}
