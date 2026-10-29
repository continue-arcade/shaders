use crate::{Registry, Result, VariantParameters, invalid, sources};
use serde_json::{Map, Value};
use std::{collections::BTreeSet, path::Path};

fn object<'a>(value: &'a Value, location: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| invalid(format!("UI 对象缺失: {location}")))
}

fn localized(value: &Value, default_locale: &str, location: &str) -> Result<()> {
    let map = object(value, location)?;
    if !map.contains_key(default_locale) {
        return Err(invalid(format!(
            "UI 缺少默认语言 {default_locale}: {location}"
        )));
    }
    for (locale, text) in map {
        if locale.trim().is_empty() || text.as_str().is_none_or(|text| text.trim().is_empty()) {
            return Err(invalid(format!(
                "UI 语言映射必须使用非空语言标签和文案: {location}"
            )));
        }
    }
    Ok(())
}

fn presentation(value: &Value, default_locale: &str, location: &str) -> Result<()> {
    localized(
        &value["label"],
        default_locale,
        &format!("{location}.label"),
    )?;
    localized(
        &value["description"],
        default_locale,
        &format!("{location}.description"),
    )?;
    Ok(())
}

/// UI 为声明数据；这里只检查关联关系和默认语言，不内置具体语言或翻译。
pub(crate) fn describe(
    root: &Path,
    registry: &Registry,
    variants: &[VariantParameters],
) -> Result<Value> {
    let bytes = sources::checked_read(root, &registry.ui_file.path)?;
    if sources::sha256(&bytes) != registry.ui_file.sha256 {
        return Err(invalid("UI 输入摘要不符"));
    }
    let ui: Value = serde_json::from_slice(&bytes)?;
    if ui["schema"] != "continue.shader-ui.v1" {
        return Err(invalid("UI schema 无效"));
    }
    let default_locale = ui["default_locale"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid("UI 缺少 default_locale"))?;
    presentation(&ui, default_locale, "family")?;
    let groups = object(&ui["groups"], "groups")?;
    if groups.is_empty() {
        return Err(invalid("UI groups 不能为空"));
    }
    for (id, group) in groups {
        presentation(group, default_locale, &format!("groups.{id}"))?;
    }
    let expected_variants: BTreeSet<_> = variants.iter().map(|v| v.variant.as_str()).collect();
    let declared_variants = object(&ui["variants"], "variants")?;
    if declared_variants
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_variants
    {
        return Err(invalid("UI 变体 ID 与解析结果不符"));
    }
    for (id, variant) in declared_variants {
        presentation(variant, default_locale, &format!("variants.{id}"))?;
    }
    let expected_parameters: BTreeSet<_> = variants
        .iter()
        .flat_map(|v| v.parameters.iter().map(|p| p.id.as_str()))
        .collect();
    let parameters = object(&ui["parameters"], "parameters")?;
    if parameters
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_parameters
    {
        return Err(invalid("UI 存在未知参数或参数文案缺失"));
    }
    for (id, parameter) in parameters {
        presentation(parameter, default_locale, &format!("parameters.{id}"))?;
        let group = parameter["group"]
            .as_str()
            .ok_or_else(|| invalid(format!("UI 参数缺少 group: {id}")))?;
        if !groups.contains_key(group) {
            return Err(invalid(format!("UI 参数引用未知分组: {id} -> {group}")));
        }
        if parameter["control"] != "number" {
            return Err(invalid(format!(
                "UI 当前参数必须显式使用 number 控件: {id}"
            )));
        }
        if let Some(options) = parameter.get("options") {
            let options = options
                .as_array()
                .ok_or_else(|| invalid(format!("UI options 必须为数组: {id}")))?;
            for option in options {
                localized(
                    &option["label"],
                    default_locale,
                    &format!("parameters.{id}.options.label"),
                )?;
                if let Some(description) = option.get("description") {
                    localized(
                        description,
                        default_locale,
                        &format!("parameters.{id}.options.description"),
                    )?;
                }
            }
        }
    }
    Ok(ui)
}
