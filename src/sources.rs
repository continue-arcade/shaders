use crate::{Error, Registry, Result, SOURCE_COMMIT, VERSION, invalid};
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
        || path.split('/').any(|s| s.is_empty())
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

pub(crate) fn validate_registry(registry: &Registry) -> Result<()> {
    if registry.schema != "continue.shader-family.v1"
        || registry.family != "zfast-crt"
        || registry.version != VERSION
        || registry.commit != SOURCE_COMMIT
        || registry.repository != "https://github.com/libretro/slang-shaders"
        || registry.license != "GPL-2.0-or-later"
    {
        return Err(invalid("登记身份或来源与首个固定切片不符"));
    }
    let variants: BTreeSet<_> = registry.variants.iter().map(|v| v.id.as_str()).collect();
    if variants != BTreeSet::from(["standard", "composite"]) || registry.variants.len() != 2 {
        return Err(invalid("必须登记 standard 与 composite 两个变体"));
    }
    for v in &registry.variants {
        relative(&v.entrypoint)?;
    }
    Ok(())
}

pub(crate) fn load(root: &Path, registry: &Registry) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut payload = BTreeMap::new();
    for file in &registry.files {
        relative(&file.path)?;
        let bytes = checked_read(root, &format!("upstream/slang-shaders/{}", file.path))?;
        if sha256(&bytes) != file.sha256 {
            return Err(invalid(format!("来源摘要不符: {}", file.path)));
        }
        if payload.insert(file.path.clone(), bytes).is_some() {
            return Err(invalid("重复来源路径"));
        }
    }
    // 在调用会自行读文件的上游库之前，先检查所有引用均已登记。
    for (path, bytes) in &payload {
        let text = std::str::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?;
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line
                .strip_prefix("#include ")
                .or_else(|| line.strip_prefix("#pragma include_optional"))
            {
                let value = rest
                    .split("//")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"');
                check_reference(path, value, &payload)?;
            } else if line.starts_with("#reference") {
                return Err(invalid("首个切片不允许 preset #reference"));
            }
            if path.ends_with(".slangp")
                && let Some((key, value)) = line.split_once('=')
            {
                let key = key.trim();
                if key
                    .strip_prefix("shader")
                    .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                {
                    check_reference(path, value.trim().trim_matches('"'), &payload)?;
                }
                if key == "textures" || key == "#reference" {
                    return Err(invalid("首个切片不允许外部纹理或引用"));
                }
            }
        }
    }
    for variant in &registry.variants {
        if !payload.contains_key(&variant.entrypoint) {
            return Err(invalid("变体入口未登记"));
        }
    }
    for variant in &registry.variants {
        dependency_closure(root, &variant.entrypoint)?;
    }
    let license = checked_read(root, &registry.license_file.path)?;
    if sha256(&license) != registry.license_file.sha256 {
        return Err(invalid("许可证摘要不符"));
    }
    payload.insert("LICENSES/GPL-2.0-or-later.txt".into(), license);
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

/// 返回真实引用闭包，并在上游预处理器递归前拒绝 include 环。
pub(crate) fn dependency_closure(root: &Path, entry: &str) -> Result<BTreeSet<String>> {
    fn visit(
        root: &Path,
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
        let text = fs::read_to_string(root.join("upstream/slang-shaders").join(entry))?;
        for line in text.lines() {
            let line = line.trim();
            let reference = line
                .strip_prefix("#include ")
                .or_else(|| line.strip_prefix("#pragma include_optional"))
                .map(|s| s.split("//").next().unwrap_or("").trim().trim_matches('"'))
                .or_else(|| {
                    if entry.ends_with(".slangp") {
                        line.split_once('=').and_then(|(key, value)| {
                            if key.trim().strip_prefix("shader").is_some_and(|s| {
                                !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
                            }) {
                                Some(value.trim().trim_matches('"'))
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                });
            if let Some(reference) = reference {
                relative(reference)?;
                let path = Path::new(entry)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(reference);
                let path = path.to_str().ok_or_else(|| invalid("引用不是 UTF-8"))?;
                visit(root, path, active, done)?;
            }
        }
        active.remove(entry);
        done.insert(entry.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    visit(root, entry, &mut BTreeSet::new(), &mut done)?;
    Ok(done)
}
