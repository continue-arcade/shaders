//! 固定来源的 Shader Cart 生成器。解析与打包不代表 GPU 渲染验收。
//!
//! UI 文案采用语言标签映射。输出 `languages` 从实际映射派生，只表示至少存在一项
//! 翻译；局部语言允许缺少其他翻译，消费者应按 `default_locale` 读取回退文案。
//! 新增语言只需更新登记的 UI 数据及其摘要，无须修改生成器。
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
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    family: String,
    version: String,
    repository: String,
    commit: String,
    license: String,
    source_root: String,
    cart_id: String,
    resource_id: String,
    tag_prefix: String,
    release_notes: String,
    packaging: Packaging,
    copyright: String,
    contributions: Vec<String>,
    license_scope: String,
    variants: Vec<Variant>,
    files: Vec<SourceFile>,
    license_file: LicenseFile,
    ui_file: SourceFile,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Packaging {
    cart_name: String,
    resource_name: String,
    author: String,
    cart_description: String,
    resource_description: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Variant {
    id: String,
    entrypoint: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LicenseFile {
    path: String,
    sha256: String,
    source: String,
}

/// 可供 CI 与分发冒烟读取的构建报告。
#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub family: String,
    pub resource_id: String,
    pub tag_prefix: String,
    pub tag: String,
    pub release_notes: String,
    pub cart_path: PathBuf,
    pub bundle_sha256: String,
    pub cart_id: String,
    pub version: String,
    pub source_commit: String,
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
pub fn build_family(root: &Path, family: &str, out: &Path) -> Result<BuildReport> {
    let registry = read_registry(root, family)?;
    let inputs = sources::load(root, &registry)?;
    // 上游解析器只接触已校验字节的临时快照，避免来源校验后的重读竞态。
    let snapshot = tempfile::tempdir()?;
    for (name, bytes) in &inputs {
        let path = snapshot.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
    }
    let variants = parameters::extract(snapshot.path(), &registry)?;
    fs::create_dir_all(out)?;
    let tree = out.join(family);
    if tree.exists() {
        return Err(invalid("输出树已存在，请使用干净输出目录"));
    }
    let cart_path = out.join(format!("{family}-{}.cart", registry.version));
    let report_path = out.join(format!("{family}-report.json"));
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
        package::cart_manifest(&registry).into_bytes(),
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
        family: family.into(),
        resource_id: registry.resource_id.clone(),
        tag_prefix: registry.tag_prefix.clone(),
        tag: format!("{}v{}", registry.tag_prefix, registry.version),
        release_notes: registry.release_notes.clone(),
        cart_id: registry.cart_id.clone(),
        version: registry.version.clone(),
        source_commit: registry.commit.clone(),
        ui_sha256: registry.ui_file.sha256.clone(),
        variants: registry.variants.iter().map(|v| v.id.clone()).collect(),
    };
    fs::write(&cart_path, archive)?;
    fs::write(report_path, json_bytes(&report)?)?;
    Ok(report)
}

fn read_registry(root: &Path, family: &str) -> Result<Registry> {
    sources::family_slug(family)?;
    let bytes = sources::checked_read(root, &format!("registry/{family}.toml"))?;
    let registry: Registry =
        toml::from_str(std::str::from_utf8(&bytes).map_err(|e| invalid(e.to_string()))?)?;
    sources::validate_registry(&registry)?;
    if registry.family != family {
        return Err(invalid("登记家族与请求不符"));
    }
    Ok(registry)
}

/// 从登记目录发现家族；供 CI 使用，不维护重复的家族列表。
pub fn families(root: &Path) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root.join("registry"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let family = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| invalid("家族不是 UTF-8"))?;
        read_registry(root, family)?;
        result.push(family.to_owned());
    }
    result.sort();
    if result.is_empty() {
        return Err(invalid("没有登记家族"));
    }
    Ok(result)
}
