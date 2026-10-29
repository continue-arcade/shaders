use crate::{Error, Registry, Result, invalid};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn relative(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == ".." || s.chars().any(char::is_control))
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid(format!("拒绝包外或非规范路径: {path}")));
    }
    Ok(())
}

pub(crate) fn checked_read(root: &Path, path: &str) -> Result<Vec<u8>> {
    relative(path)?;
    let mut cursor = PathBuf::from(root);
    for component in Path::new(path).components() {
        cursor.push(component);
        if fs::symlink_metadata(&cursor)?.file_type().is_symlink() {
            return Err(invalid(format!("拒绝符号链接: {path}")));
        }
    }
    Ok(fs::read(cursor)?)
}

pub(crate) fn family_slug(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 64
        || value.split('-').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
        return Err(invalid(
            "家族/变体 ID 必须是小写字母、数字和单连字符组成的 slug",
        ));
    }
    Ok(())
}

fn digest(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn validate_registry(r: &Registry) -> Result<()> {
    family_slug(&r.family)?;
    if r.schema != "continue.shader-family.v1"
        || semver::Version::parse(&r.version).is_err()
        || !digest(&r.commit, 40)
        || !r.repository.starts_with("https://github.com/")
        || r.repository.split('/').skip(3).count() != 2
        || r.repository.split('/').skip(3).any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
        || r.cart_id != format!("org.continue.shaders.{}", r.family)
        || r.resource_id != format!("org.continue.shader.{}", r.family)
        || r.tag_prefix != format!("{}/", r.family)
        || r.license.trim().is_empty()
    {
        return Err(invalid("登记身份、SemVer、tag 或固定来源无效"));
    }
    relative(&r.source_root)?;
    if !r.source_root.starts_with("upstream/") {
        return Err(invalid("来源根必须位于 upstream/"));
    }
    relative(&r.release_notes)?;
    if r.release_notes != format!("docs/releases/{}-{}.md", r.family, r.version) {
        return Err(invalid("更新说明路径必须匹配家族与版本"));
    }
    let mut ids = BTreeSet::new();
    if r.variants.is_empty() {
        return Err(invalid("家族变体不能为空"));
    }
    for v in &r.variants {
        family_slug(&v.id)?;
        if v.id == "family" {
            return Err(invalid("变体 ID family 是参数摘要的保留名称"));
        }
        relative(&v.entrypoint)?;
        if !v.entrypoint.ends_with(".slangp") || !ids.insert(&v.id) {
            return Err(invalid("入口类型或重复变体无效"));
        }
    }
    for f in &r.files {
        if [
            "continue.toml",
            "cartfile.toml",
            "ui.json",
            "provenance.json",
        ]
        .contains(&f.path.as_str())
            || f.path.starts_with("parameters/")
            || f.path.starts_with("LICENSES/")
        {
            return Err(invalid("来源路径不能占用生成元数据的保留名称"));
        }
    }
    for f in r.files.iter().chain([&r.ui_file]) {
        relative(&f.path)?;
        if !digest(&f.sha256, 64) {
            return Err(invalid("文件 SHA-256 必须是 64 位小写十六进制"));
        }
    }
    relative(&r.license_file.path)?;
    if !r.license_file.path.starts_with("LICENSES/") || !digest(&r.license_file.sha256, 64) {
        return Err(invalid("许可路径或摘要无效"));
    }
    if r.ui_file.path != format!("registry/{}.ui.json", r.family) {
        return Err(invalid("UI 文件必须属于本家族"));
    }
    Ok(())
}

pub(crate) fn load(root: &Path, registry: &Registry) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut payload = BTreeMap::new();
    for file in &registry.files {
        relative(&file.path)?;
        let bytes = checked_read(root, &format!("{}/{}", registry.source_root, file.path))?;
        if sha256(&bytes) != file.sha256 {
            return Err(invalid(format!("来源摘要不符: {}", file.path)));
        }
        if payload.insert(file.path.clone(), bytes).is_some() {
            return Err(invalid("重复来源路径"));
        }
    }
    // 在上游解析器接触磁盘前，只接受当前支持的直接 preset 词法。
    // 上游会推断隐式纹理并吞掉块注释；不可让这些宽容行为绕过登记闭包。
    for (path, bytes) in &payload {
        let text = std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?;
        for reference in references(path, text)? {
            check_reference(path, reference, &payload)?;
        }
    }
    for variant in &registry.variants {
        if !payload.contains_key(&variant.entrypoint) {
            return Err(invalid("变体入口未登记"));
        }
    }
    let mut closure = BTreeSet::new();
    for variant in &registry.variants {
        closure.extend(payload_closure(&payload, &variant.entrypoint)?);
    }
    if closure != payload.keys().cloned().collect() {
        return Err(invalid(
            "来源登记必须准确覆盖入口依赖闭包，不允许未引用文件",
        ));
    }
    let license = checked_read(root, &registry.license_file.path)?;
    if sha256(&license) != registry.license_file.sha256 {
        return Err(invalid("许可证摘要不符"));
    }
    if payload
        .insert(registry.license_file.path.clone(), license)
        .is_some()
    {
        return Err(invalid("许可路径与来源冲突"));
    }
    Ok(payload)
}

pub(crate) fn check_reference(
    from: &str,
    value: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    relative(value)?;
    let target = Path::new(from)
        .parent()
        .unwrap_or(Path::new(""))
        .join(value);
    let name = target
        .to_str()
        .ok_or_else(|| Error::Invalid("引用路径不是 UTF-8".into()))?;
    if !files.contains_key(name) {
        return Err(invalid(format!("未登记依赖: {from} -> {value}")));
    }
    Ok(())
}

fn indexed(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix)
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

/// 当前 preset profile 刻意保守，新增语法必须有闭包测试后再开放。
fn references<'a>(path: &str, text: &'a str) -> Result<Vec<&'a str>> {
    let mut result = Vec::new();
    let preset = path.ends_with(".slangp");
    if preset && (text.contains("/*") || text.contains("*/")) {
        return Err(invalid("当前 preset profile 不允许块注释"));
    }
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("#pragma include_optional") {
            return Err(invalid("当前闭包 profile 不允许 optional include"));
        }
        if let Some(rest) = line.strip_prefix("#include ") {
            result.push(
                rest.split("//")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"'),
            );
        } else if line.starts_with("#reference") {
            return Err(invalid("当前 preset profile 不允许 #reference"));
        } else if preset {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| invalid("preset 行缺少等号"))?;
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            // 拒绝同行多条语句、引号逃逸以及上游注释处理差异。
            if key.is_empty()
                || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || value.contains('"')
                || value.contains("//")
            {
                return Err(invalid("preset key/value 词法超出受支持 profile"));
            }
            if indexed(key, "shader") {
                result.push(value);
            } else if key == "parameters" {
                if value.split(';').any(|id| {
                    id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                }) {
                    return Err(invalid("preset 参数列表无效"));
                }
            } else if [
                "filter_linear",
                "mipmap_input",
                "float_framebuffer",
                "srgb_framebuffer",
            ]
            .iter()
            .any(|prefix| indexed(key, prefix))
            {
                if !["true", "false"].contains(&value) {
                    return Err(invalid("preset 布尔值无效"));
                }
            } else if ["scale_type", "scale_type_x", "scale_type_y"]
                .iter()
                .any(|prefix| indexed(key, prefix))
            {
                if !["source", "viewport", "absolute"].contains(&value) {
                    return Err(invalid("preset 缩放类型无效"));
                }
            } else if indexed(key, "wrap_mode") {
                if ![
                    "clamp_to_border",
                    "clamp_to_edge",
                    "repeat",
                    "mirrored_repeat",
                ]
                .contains(&value)
                {
                    return Err(invalid("preset wrap mode 无效"));
                }
            } else if indexed(key, "alias") {
                if value.is_empty()
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(invalid("preset alias 无效"));
                }
            } else if value.parse::<f32>().is_err() || key == "textures" {
                return Err(invalid(format!(
                    "preset 不支持的键值（禁止隐式纹理）: {key}"
                )));
            }
        }
    }
    Ok(result)
}

/// 只从已校验的内存字节计算闭包，不重读可变来源目录。
fn payload_closure(payload: &BTreeMap<String, Vec<u8>>, entry: &str) -> Result<BTreeSet<String>> {
    fn visit(
        payload: &BTreeMap<String, Vec<u8>>,
        entry: &str,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
    ) -> Result<()> {
        if done.contains(entry) {
            return Ok(());
        }
        if !active.insert(entry.into()) {
            return Err(invalid(format!("依赖引用形成环: {entry}")));
        }
        let bytes = payload
            .get(entry)
            .ok_or_else(|| invalid(format!("未登记依赖: {entry}")))?;
        for value in references(
            entry,
            std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?,
        )? {
            relative(value)?;
            let path = Path::new(entry)
                .parent()
                .unwrap_or(Path::new(""))
                .join(value);
            let path = path.to_str().ok_or_else(|| invalid("引用不是 UTF-8"))?;
            visit(payload, path, active, done)?;
        }
        active.remove(entry);
        done.insert(entry.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    visit(payload, entry, &mut BTreeSet::new(), &mut done)?;
    Ok(done)
}

/// 从隔离 snapshot 读取 ShaderSource 的参数位置；该目录只有已校验字节。
pub(crate) fn dependency_closure(root: &Path, entry: &str) -> Result<BTreeSet<String>> {
    fn read(root: &Path, entry: &str, payload: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        if payload.contains_key(entry) {
            return Ok(());
        }
        let bytes = checked_read(root, entry)?;
        let text = std::str::from_utf8(&bytes).map_err(|e| invalid(e.to_string()))?;
        let refs = references(entry, text)?
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        payload.insert(entry.into(), bytes);
        for value in refs {
            relative(&value)?;
            let path = Path::new(entry)
                .parent()
                .unwrap_or(Path::new(""))
                .join(value);
            read(
                root,
                path.to_str().ok_or_else(|| invalid("引用不是 UTF-8"))?,
                payload,
            )?;
        }
        Ok(())
    }
    let mut payload = BTreeMap::new();
    read(root, entry, &mut payload)?;
    payload_closure(&payload, entry)
}
