//! 固定来源的 Shader Cart 生成器。解析与打包不代表 GPU 渲染验收。
#![forbid(unsafe_code)]

mod package;
mod parameters;
mod sources;
mod ui;

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub use parameters::{Parameter, VariantParameters, parse_overrides};

/// 首个家族的 Cart 身份。
pub const CART_ID: &str = "org.continue.shaders.zfast-crt";
/// 固定的冒烟候选版本。
pub const VERSION: &str = "0.1.0-smoke.1";
/// 上游固定快照。
pub const SOURCE_COMMIT: &str = "2b0530bac0e8083d9d643c194929dc0a6b69bd35";

/// 构建过程中可恢复的错误。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("文件操作失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("登记 TOML 错误: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP 错误: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Invalid(String),
}
/// 生成器结果。
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize)]
struct Registry {
    schema: String,
    family: String,
    version: String,
    repository: String,
    commit: String,
    license: String,
    variants: Vec<Variant>,
    files: Vec<SourceFile>,
    license_file: LicenseFile,
    ui_file: SourceFile,
}
#[derive(Debug, Deserialize, Serialize)]
struct Variant {
    id: String,
    entrypoint: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct SourceFile {
    path: String,
    sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct LicenseFile {
    path: String,
    sha256: String,
    source: String,
}

/// 可供 CI 与分发冒烟读取的构建报告。
#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub cart_path: PathBuf,
    pub bundle_sha256: String,
    pub cart_id: &'static str,
    pub version: &'static str,
    pub source_commit: &'static str,
    pub ui_sha256: String,
    pub variants: Vec<String>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut data = serde_json::to_vec_pretty(value)?;
    data.push(b'\n');
    Ok(data)
}

/// 从仓库内登记的来源离线构建，输出展开树、Cart 和 JSON 报告。
/// 输出树必须不存在，避免旧文件混入或覆盖已有候选。
pub fn build(root: &Path, out: &Path) -> Result<BuildReport> {
    let registry: Registry =
        toml::from_str(&fs::read_to_string(root.join("registry/zfast-crt.toml"))?)?;
    sources::validate_registry(&registry)?;
    let inputs = sources::load(root, &registry)?;
    let variants = parameters::extract(root, &registry)?;
    fs::create_dir_all(out)?;
    let tree = out.join("zfast-crt");
    if tree.exists() {
        return Err(invalid("输出树已存在，请使用干净输出目录"));
    }
    let cart_path = out.join(format!("zfast-crt-{VERSION}.cart"));
    let report_path = out.join("zfast-crt-report.json");
    if cart_path.exists() || report_path.exists() {
        return Err(invalid("输出候选或报告已存在"));
    }
    let mut payload = inputs;
    for variant in &variants {
        payload.insert(
            format!("parameters/{}.json", variant.variant),
            json_bytes(variant)?,
        );
    }
    payload.insert(
        "parameters/family.json".into(),
        json_bytes(&parameters::family_description(&variants)?)?,
    );
    payload.insert(
        "ui.json".into(),
        json_bytes(&ui::describe(root, &registry, &variants)?)?,
    );
    payload.insert(
        "continue.toml".into(),
        package::resource_manifest(&registry).into_bytes(),
    );
    payload.insert(
        "cartfile.toml".into(),
        package::cart_manifest().into_bytes(),
    );
    package::add_provenance(&mut payload, &registry)?;
    // 输出前已完成来源校验与解析；所有输出名均来自固定生成路径或验证过的登记。
    fs::create_dir(&tree)?;
    for (name, bytes) in &payload {
        let dest = tree.join(name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, bytes)?;
    }
    let archive = package::archive(&payload)?;
    let report = BuildReport {
        cart_path: fs::canonicalize(out)?.join(
            cart_path
                .file_name()
                .ok_or_else(|| invalid("产物路径错误"))?,
        ),
        bundle_sha256: format!("sha256:{}", sources::sha256(&archive)),
        cart_id: CART_ID,
        version: VERSION,
        source_commit: SOURCE_COMMIT,
        ui_sha256: registry.ui_file.sha256.clone(),
        variants: registry.variants.iter().map(|v| v.id.clone()).collect(),
    };
    fs::write(&cart_path, archive)?;
    fs::write(report_path, json_bytes(&report)?)?;
    Ok(report)
}
