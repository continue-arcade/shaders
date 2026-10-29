use crate::{Registry, Result, invalid};
use librashader_preprocess::ShaderSource;
use librashader_presets::{ShaderFeatures, ShaderPreset};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

/// 参数数值来自固定版本 librashader；ID 保持上游原值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    pub id: String,
    pub upstream_description: String,
    pub source_initial: f32,
    pub variant_default: f32,
    pub preset_override: Option<f32>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub source: String,
    pub source_line: usize,
    pub preset: String,
    pub pass: usize,
}
/// 一个真实 preset 的完整参数集合。
#[derive(Debug, Serialize, Deserialize)]
pub struct VariantParameters {
    pub schema: String,
    pub variant: String,
    pub entrypoint: String,
    pub parser: String,
    pub parameters: Vec<Parameter>,
}

pub(crate) fn extract(root: &Path, registry: &Registry) -> Result<Vec<VariantParameters>> {
    let source_root = fs::canonicalize(root.join("upstream/slang-shaders"))?;
    let mut results = Vec::new();
    for variant in &registry.variants {
        let preset =
            ShaderPreset::try_parse(source_root.join(&variant.entrypoint), ShaderFeatures::NONE)
                .map_err(|e| invalid(format!("preset 解析失败: {e}")))?;
        let mut parameters: BTreeMap<String, Parameter> = BTreeMap::new();
        for (pass_id, pass) in preset.passes.iter().enumerate() {
            let pass_path = fs::canonicalize(&pass.path)?;
            let pass_relative = pass_path
                .strip_prefix(&source_root)
                .map_err(|_| invalid("解析出的 pass 位于包外"))?
                .to_str()
                .ok_or_else(|| invalid("pass 路径不是 UTF-8"))?;
            if !registry.files.iter().any(|f| f.path == pass_relative) {
                return Err(invalid("解析出的 pass 未登记"));
            }
            let source = ShaderSource::load(&pass_path, ShaderFeatures::NONE)
                .map_err(|e| invalid(format!("Shader 预处理失败: {e}")))?;
            for parameter in source.parameters.values() {
                let id = parameter.id.to_string();
                let closure = crate::sources::dependency_closure(root, pass_relative)?;
                let mut locations = Vec::new();
                for path in closure {
                    let text = fs::read_to_string(source_root.join(&path))?;
                    for (line_id, line) in text.lines().enumerate() {
                        if line
                            .trim()
                            .strip_prefix("#pragma parameter ")
                            .and_then(|s| s.split_whitespace().next())
                            == Some(id.as_str())
                        {
                            locations.push((path.clone(), line_id + 1));
                        }
                    }
                }
                if locations.len() != 1 {
                    return Err(invalid(format!("参数 {id} 来源不唯一或缺失")));
                }
                let (source_file, source_line) = locations.remove(0);
                let preset_override = preset
                    .parameters
                    .iter()
                    .find(|p| p.name.as_str() == id)
                    .map(|p| p.value);
                let item = Parameter {
                    id: id.clone(),
                    upstream_description: parameter.description.clone(),
                    source_initial: parameter.initial,
                    variant_default: preset_override.unwrap_or(parameter.initial),
                    preset_override,
                    min: parameter.minimum,
                    max: parameter.maximum,
                    step: parameter.step,
                    source: source_file,
                    source_line,
                    preset: variant.entrypoint.clone(),
                    pass: pass_id,
                };
                validate_parameter(&item)?;
                if let Some(existing) = parameters.get(&id) {
                    if existing.source_initial != item.source_initial
                        || existing.variant_default != item.variant_default
                        || existing.min != item.min
                        || existing.max != item.max
                        || existing.step != item.step
                    {
                        return Err(invalid(format!("参数 {id} 跨 pass 定义冲突")));
                    }
                } else {
                    parameters.insert(id, item);
                }
            }
        }
        for parameter in &preset.parameters {
            if !parameters.contains_key(parameter.name.as_str()) {
                return Err(invalid(format!("未知 preset 参数: {}", parameter.name)));
            }
        }
        results.push(VariantParameters {
            schema: "continue.shader-parameters.v1".into(),
            variant: variant.id.clone(),
            entrypoint: variant.entrypoint.clone(),
            parser: "librashader-presets=0.12.1;librashader-preprocess=0.12.1".into(),
            parameters: parameters.into_values().collect(),
        });
    }
    Ok(results)
}

fn validate_parameter(p: &Parameter) -> Result<()> {
    if ![p.source_initial, p.variant_default, p.min, p.max, p.step]
        .iter()
        .all(|v| v.is_finite())
        || p.min > p.max
        || p.step <= 0.0
        || p.source_initial < p.min
        || p.source_initial > p.max
        || p.variant_default < p.min
        || p.variant_default > p.max
    {
        return Err(invalid(format!("参数范围或默认值无效: {}", p.id)));
    }
    Ok(())
}

/// 校验用户数值覆盖；不写入 Forge，也不自动把浮点数猜为布尔值。
/// 不要求步长整除：step 是上游 UI 增量，取值域由 min/max 决定。
pub fn parse_overrides(
    parameters: &[Parameter],
    values: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, f32>> {
    values
        .iter()
        .map(|(id, value)| {
            let parameter = parameters
                .iter()
                .find(|p| &p.id == id)
                .ok_or_else(|| invalid(format!("未知参数: {id}")))?;
            let parsed: f32 = value
                .parse()
                .map_err(|_| invalid(format!("参数不是数值: {id}")))?;
            if !parsed.is_finite() || parsed < parameter.min || parsed > parameter.max {
                return Err(invalid(format!("参数非有限或越界: {id}")));
            }
            Ok((id.clone(), parsed))
        })
        .collect()
}

/// 合并家族参数 ID，但不抹去变体之间的来源值、范围或默认值差异。
pub(crate) fn family_description(variants: &[VariantParameters]) -> Result<serde_json::Value> {
    use serde_json::{Value, json};
    let mut merged: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    for variant in variants {
        for p in &variant.parameters {
            merged
                .entry(p.id.clone())
                .or_default()
                .insert(variant.variant.clone(), serde_json::to_value(p)?);
        }
    }
    let mut parameters = BTreeMap::new();
    for (id, definitions) in merged {
        let mut differing_fields = Vec::new();
        for field in [
            "upstream_description",
            "source_initial",
            "variant_default",
            "min",
            "max",
            "step",
        ] {
            if let Some(first) = definitions.values().next()
                && definitions
                    .values()
                    .any(|value| value[field] != first[field])
            {
                differing_fields.push(field);
            }
        }
        parameters.insert(
            id,
            json!({"variants":definitions,"differing_fields":differing_fields}),
        );
    }
    Ok(
        json!({"schema":"continue.shader-family-parameters.v1","parameters":parameters,"difference_policy":"Variant differences are explicitly preserved; conflicting definitions within one preset are rejected."}),
    )
}
