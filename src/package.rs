use crate::{CART_ID, Registry, Result, VERSION, json_bytes, sources::sha256};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};
use zip::{ZipWriter, write::SimpleFileOptions};

pub(crate) fn cart_manifest() -> String {
    format!(
        r#"schema = "continue.cartfile.v1"
id = "{CART_ID}"
name = "Zfast CRT Shader Family"
version = "{VERSION}"
author = "Greg Hogan (SoltanGris42); metallic77"
publisher = "Continue"
description = "Self-contained source-locked Zfast CRT family; distribution smoke candidate."

[links]
source = "https://github.com/libretro/slang-shaders"

[artifact]
format = "cart.zip.v1"
[artifact.target]
runtime = "continue"

[include]
paths = ["continue.toml", "crt/", "parameters/", "ui.json", "provenance.json", "LICENSES/"]
"#
    )
}

pub(crate) fn resource_manifest(registry: &Registry) -> String {
    let mut output = format!(
        r#"schema = "continue.resource.v1"
type = "shader"
id = "org.continue.shader.zfast-crt"
name = "Zfast CRT"
version = "{VERSION}"
author = "Greg Hogan (SoltanGris42); metallic77"
license = "GPL-2.0-or-later"
description = "Zfast CRT standard and composite presets. GPU rendering validation pending."
entrypoint = "crt/zfast-crt.slangp"

[[targets]]
runtime = "continue"

[shader]
format = "slang"
ui = "ui.json"
"#
    );
    for v in &registry.variants {
        output.push_str(&format!(
            "\n[shader.variants.{}]\nentrypoint = \"{}\"\nparameters = \"parameters/{}.json\"\n",
            v.id, v.entrypoint, v.id
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
        "copyright": "Copyright (C) 2017 Greg Hogan (SoltanGris42)",
        "contributions": ["GTUv050-alike blur added by metallic77"],
        "license_scope": "GPL-2.0-or-later applies to the bundled shader family and Cart. The generator in the repository is MIT licensed.",
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
