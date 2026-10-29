use crate::{Registry, Result, json_bytes, sources::sha256};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};
use zip::{ZipWriter, write::SimpleFileOptions};

pub(crate) fn cart_manifest(r: &Registry) -> String {
    let cart_id = &r.cart_id;
    let version = &r.version;
    let p = &r.packaging;
    let roots: std::collections::BTreeSet<_> = r
        .files
        .iter()
        .map(|f| {
            f.path
                .split_once('/')
                .map_or_else(|| f.path.clone(), |(dir, _)| format!("{dir}/"))
        })
        .collect();
    let mut paths = vec!["continue.toml".to_owned()];
    paths.extend(roots);
    paths.extend(["parameters/", "ui.json", "provenance.json", "LICENSES/"].map(str::to_owned));
    let paths = paths
        .iter()
        .map(|v| toml::Value::String(v.clone()).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"schema = "continue.cartfile.v1"
id = "{cart_id}"
name = {cart_name}
version = "{version}"
author = {author}
publisher = "Continue"
description = {description}

[links]
source = {repository}

[artifact]
format = "cart.zip.v1"
[artifact.target]
runtime = "continue"

[include]
paths = [{paths}]
"#,
        cart_name = quote(&p.cart_name),
        author = quote(&p.author),
        description = quote(&p.cart_description),
        repository = quote(&r.repository),
    )
}

pub(crate) fn resource_manifest(registry: &Registry) -> String {
    let version = &registry.version;
    let mut output = format!(
        r#"schema = "continue.resource.v1"
type = "shader"
id = {resource_id}
name = {resource_name}
version = "{version}"
author = {author}
license = {license}
description = {description}
entrypoint = {entrypoint}

[[targets]]
runtime = "continue"

[shader]
format = "slang"
ui = "ui.json"
"#,
        resource_id = quote(&registry.resource_id),
        resource_name = quote(&registry.packaging.resource_name),
        author = quote(&registry.packaging.author),
        license = quote(&registry.license),
        description = quote(&registry.packaging.resource_description),
        entrypoint = quote(&registry.variants[0].entrypoint),
    );
    for v in &registry.variants {
        output.push_str(&format!(
            "\n[shader.variants.{}]\nentrypoint = {}\nparameters = \"parameters/{}.json\"\n",
            v.id,
            quote(&v.entrypoint),
            v.id
        ));
    }
    output
}

pub(crate) fn add_provenance(
    payload: &mut BTreeMap<String, Vec<u8>>,
    registry: &Registry,
) -> Result<()> {
    let files: BTreeMap<_, _> = payload
        .iter()
        .map(|(name, bytes)| (name, json!({"sha256": sha256(bytes), "size": bytes.len()})))
        .collect();
    let provenance = json!({
        "schema": "continue.shader-provenance.v1",
        "family": registry.family,
        "repository": registry.repository,
        "commit": registry.commit,
        "source_files": registry.files,
        "metadata_inputs": {"ui": registry.ui_file},
        "license": registry.license,
        "license_file": registry.license_file,
        "copyright": registry.copyright,
        "contributions": registry.contributions,
        "license_scope": registry.license_scope,
        "parser": {"librashader-presets": "0.12.1", "librashader-preprocess": "0.12.1"},
        "files": files,
        "inventory_scope": "All packaged files except provenance.json itself; the complete archive SHA-256 is in the external build report.",
        "validation": {"preset_parse": true, "include_preprocess": true, "gpu_render": false}
    });
    payload.insert("provenance.json".into(), json_bytes(&provenance)?);
    Ok(())
}

pub(crate) fn archive(payload: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for (name, bytes) in payload {
        zip.start_file(name, options)?;
        zip.write_all(bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

fn quote(value: &str) -> String {
    toml::Value::String(value.into()).to_string()
}
