use std::{fs, path::Path};

#[test]
fn generates_two_variants_from_locked_sources() {
    let out = tempfile::tempdir().unwrap();
    continue_shader_carts::build(Path::new(env!("CARGO_MANIFEST_DIR")), out.path()).unwrap();
    let standard: serde_json::Value = serde_json::from_slice(
        &fs::read(out.path().join("zfast-crt/parameters/standard.json")).unwrap(),
    )
    .unwrap();
    let composite: serde_json::Value = serde_json::from_slice(
        &fs::read(out.path().join("zfast-crt/parameters/composite.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(standard["parameters"].as_array().unwrap().len(), 6);
    assert_eq!(composite["parameters"].as_array().unwrap().len(), 10);
    let blur = standard["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "BLURSCALEX")
        .unwrap();
    assert!((blur["source_initial"].as_f64().unwrap() - 0.3).abs() < 0.0001);
    assert_eq!(blur["source"], "crt/shaders/zfast_crt/zfast_crt_impl.inc");
    assert!(out.path().join("zfast-crt-0.1.0-smoke.1.cart").exists());
}

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("registry/zfast-crt.toml")).unwrap()).unwrap();
    let mut files = vec![
        "registry/zfast-crt.toml".to_string(),
        "registry/zfast-crt.ui.json".to_string(),
        "LICENSES/GPL-2.0-or-later.txt".to_string(),
    ];
    for entry in registry["files"].as_array().unwrap() {
        files.push(format!(
            "upstream/slang-shaders/{}",
            entry["path"].as_str().unwrap()
        ));
    }
    for name in files {
        let dest = temp.path().join(&name);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(root.join(name), dest).unwrap();
    }
    temp
}

fn change_source(root: &Path, path: &str, text: &str) {
    use sha2::{Digest, Sha256};
    fs::write(root.join("upstream/slang-shaders").join(path), text).unwrap();
    let registry_path = root.join("registry/zfast-crt.toml");
    let mut registry: toml::Value =
        toml::from_str(&fs::read_to_string(&registry_path).unwrap()).unwrap();
    for entry in registry["files"].as_array_mut().unwrap() {
        if entry["path"].as_str() == Some(path) {
            entry["sha256"] = toml::Value::String(format!("{:x}", Sha256::digest(text.as_bytes())));
        }
    }
    fs::write(registry_path, toml::to_string(&registry).unwrap()).unwrap();
}

fn build_fixture(root: &Path) -> tempfile::TempDir {
    let out = tempfile::tempdir().unwrap();
    continue_shader_carts::build(root, out.path()).unwrap();
    out
}

fn params(out: &Path, variant: &str) -> continue_shader_carts::VariantParameters {
    serde_json::from_slice(
        &fs::read(out.join(format!("zfast-crt/parameters/{variant}.json"))).unwrap(),
    )
    .unwrap()
}

#[test]
fn preset_override_preserves_source_default_and_parameter_id() {
    let root = fixture();
    let path = "crt/zfast-crt.slangp";
    let mut text =
        fs::read_to_string(root.path().join("upstream/slang-shaders").join(path)).unwrap();
    text.push_str("\nparameters = \"BLURSCALEX\"\nBLURSCALEX = 0.65\n");
    change_source(root.path(), path, &text);
    let out = build_fixture(root.path());
    let parameters = params(out.path(), "standard");
    let blur = parameters
        .parameters
        .iter()
        .find(|p| p.id == "BLURSCALEX")
        .unwrap();
    assert_eq!(blur.source_initial, 0.3);
    assert_eq!(blur.variant_default, 0.65);
    assert_eq!(blur.preset_override, Some(0.65));
}

#[test]
fn original_ids_have_complete_language_maps_and_numeric_controls() {
    let out = build_fixture(Path::new(env!("CARGO_MANIFEST_DIR")));
    let ui: serde_json::Value =
        serde_json::from_slice(&fs::read(out.path().join("zfast-crt/ui.json")).unwrap()).unwrap();
    let composite = params(out.path(), "composite");
    for p in &composite.parameters {
        let entry = &ui["parameters"][&p.id];
        for field in ["label", "description"] {
            for language in ["en", "zh-CN"] {
                assert!(!entry[field][language].as_str().unwrap().is_empty());
            }
        }
        assert_eq!(entry["control"], "number");
        assert!(matches!(
            entry["group"].as_str().unwrap(),
            "basic" | "advanced"
        ));
    }
    assert_eq!(ui["parameters"].as_object().unwrap().len(), 10);
    assert!(!ui.to_string().contains("label_zh"));
}

#[test]
fn rejects_unknown_nonfinite_malformed_and_out_of_range_user_values() {
    use std::collections::BTreeMap;
    let out = build_fixture(Path::new(env!("CARGO_MANIFEST_DIR")));
    let parameters = params(out.path(), "standard").parameters;
    for (id, value) in [
        ("UNKNOWN", "0"),
        ("BLURSCALEX", "NaN"),
        ("BLURSCALEX", "inf"),
        ("BLURSCALEX", "-inf"),
        ("BLURSCALEX", "1.01"),
        ("BLURSCALEX", "-0.1"),
        ("BLURSCALEX", "text"),
    ] {
        assert!(
            continue_shader_carts::parse_overrides(
                &parameters,
                &BTreeMap::from([(id.into(), value.into())])
            )
            .is_err(),
            "{id}={value}"
        );
    }
    let valid = continue_shader_carts::parse_overrides(
        &parameters,
        &BTreeMap::from([("BLURSCALEX".into(), "0.42".into())]),
    )
    .unwrap();
    assert_eq!(valid["BLURSCALEX"], 0.42);
}

#[test]
fn same_inputs_create_identical_zip_bytes_with_fixed_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let first = build_fixture(root);
    let second = build_fixture(root);
    let name = "zfast-crt-0.1.0-smoke.1.cart";
    let a = fs::read(first.path().join(name)).unwrap();
    assert_eq!(a, fs::read(second.path().join(name)).unwrap());
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(a)).unwrap();
    let mut names = Vec::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i).unwrap();
        assert_eq!(entry.last_modified().unwrap(), zip::DateTime::default());
        assert_eq!(entry.unix_mode().unwrap() & 0o777, 0o644);
        names.push(entry.name().to_string());
    }
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn rejects_tampered_and_missing_sources_before_creating_output() {
    for missing in [false, true] {
        let root = fixture();
        let file = root
            .path()
            .join("upstream/slang-shaders/crt/shaders/zfast_crt/zfast_crt_impl.inc");
        if missing {
            fs::remove_file(file).unwrap();
        } else {
            fs::write(file, "tampered").unwrap();
        }
        let out = tempfile::tempdir().unwrap();
        assert!(continue_shader_carts::build(root.path(), out.path()).is_err());
        assert!(!out.path().join("zfast-crt").exists());
    }
}

#[test]
fn rejects_unregistered_missing_and_escape_dependencies_even_with_matching_digest() {
    for dependency in [
        "../../outside.inc",
        "/tmp/outside.inc",
        "missing.inc",
        "zfast_crt_impl.inc/../zfast_crt_impl.inc",
        "C:\\outside.inc",
    ] {
        let root = fixture();
        let path = "crt/shaders/zfast_crt/zfast_crt_finemask.slang";
        change_source(
            root.path(),
            path,
            &format!("#version 450\n#include \"{dependency}\"\n"),
        );
        let out = tempfile::tempdir().unwrap();
        assert!(
            continue_shader_carts::build(root.path(), out.path()).is_err(),
            "{dependency}"
        );
    }
}

#[test]
fn output_tree_is_independently_loadable_by_real_librashader() {
    use librashader_presets::{ShaderFeatures, ShaderPreset};
    let out = build_fixture(Path::new(env!("CARGO_MANIFEST_DIR")));
    for entry in ["zfast-crt.slangp", "zfast-crt-composite.slangp"] {
        let preset = ShaderPreset::try_parse(
            out.path().join("zfast-crt/crt").join(entry),
            ShaderFeatures::NONE,
        )
        .unwrap();
        for pass in preset.passes {
            let source =
                librashader_preprocess::ShaderSource::load(pass.path, ShaderFeatures::NONE)
                    .unwrap();
            assert!(!source.vertex.is_empty());
            assert!(!source.fragment.is_empty());
        }
    }
}

#[cfg(unix)]
#[test]
fn refuses_symlink_sources() {
    let root = fixture();
    let source = root
        .path()
        .join("upstream/slang-shaders/crt/zfast-crt.slangp");
    let moved = root.path().join("elsewhere.slangp");
    fs::rename(&source, &moved).unwrap();
    std::os::unix::fs::symlink(moved, source).unwrap();
    let out = tempfile::tempdir().unwrap();
    assert!(continue_shader_carts::build(root.path(), out.path()).is_err());
}

#[test]
fn absent_step_uses_the_pinned_librashader_default() {
    let root = fixture();
    let path = "crt/shaders/zfast_crt/zfast_crt_impl.inc";
    let text = fs::read_to_string(root.path().join("upstream/slang-shaders").join(path))
        .unwrap()
        .replace(
            "\"Blur Amount X-Axis\" 0.30 0.0 1.0 0.05",
            "\"Blur Amount X-Axis\" 0.30 0.0 1.0",
        );
    change_source(root.path(), path, &text);
    let out = build_fixture(root.path());
    let parameters = params(out.path(), "standard");
    assert_eq!(
        parameters
            .parameters
            .iter()
            .find(|p| p.id == "BLURSCALEX")
            .unwrap()
            .step,
        0.02
    );
}

#[test]
fn cyclic_includes_fail_before_upstream_preprocessor_recursion() {
    let root = fixture();
    let path = "crt/shaders/zfast_crt/zfast_crt_impl.inc";
    let mut text =
        fs::read_to_string(root.path().join("upstream/slang-shaders").join(path)).unwrap();
    text.push_str("\n#include \"zfast_crt_impl.inc\"\n");
    change_source(root.path(), path, &text);
    let out = tempfile::tempdir().unwrap();
    let err = continue_shader_carts::build(root.path(), out.path()).unwrap_err();
    assert!(err.to_string().contains("形成环"));
}

#[test]
fn provenance_inventory_matches_every_payload_file_except_itself() {
    use sha2::{Digest, Sha256};
    let out = build_fixture(Path::new(env!("CARGO_MANIFEST_DIR")));
    let tree = out.path().join("zfast-crt");
    let provenance: serde_json::Value =
        serde_json::from_slice(&fs::read(tree.join("provenance.json")).unwrap()).unwrap();
    for (path, metadata) in provenance["files"].as_object().unwrap() {
        let bytes = fs::read(tree.join(path)).unwrap();
        assert_eq!(metadata["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        assert_eq!(metadata["size"].as_u64().unwrap(), bytes.len() as u64);
    }
    let zip = zip::ZipArchive::new(
        fs::File::open(out.path().join("zfast-crt-0.1.0-smoke.1.cart")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        zip.len(),
        provenance["files"].as_object().unwrap().len() + 1
    );
}

#[test]
fn shared_parameter_ids_preserve_variant_default_differences() {
    let out = build_fixture(Path::new(env!("CARGO_MANIFEST_DIR")));
    let family: serde_json::Value = serde_json::from_slice(
        &fs::read(out.path().join("zfast-crt/parameters/family.json")).unwrap(),
    )
    .unwrap();
    let blur = &family["parameters"]["BLURSCALEX"];
    assert!(
        (blur["variants"]["standard"]["source_initial"]
            .as_f64()
            .unwrap()
            - 0.3)
            .abs()
            < 0.00001
    );
    assert_eq!(blur["variants"]["composite"]["source_initial"], 0.5);
    assert!(
        blur["differing_fields"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("source_initial"))
    );
    assert_eq!(family["parameters"].as_object().unwrap().len(), 10);
}

fn change_ui(root: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    use sha2::{Digest, Sha256};
    let path = root.join("registry/zfast-crt.ui.json");
    let mut ui: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    edit(&mut ui);
    let bytes = serde_json::to_vec_pretty(&ui).unwrap();
    fs::write(path, &bytes).unwrap();
    let registry_path = root.join("registry/zfast-crt.toml");
    let mut registry: toml::Value =
        toml::from_str(&fs::read_to_string(&registry_path).unwrap()).unwrap();
    registry["ui_file"]["sha256"] = toml::Value::String(format!("{:x}", Sha256::digest(&bytes)));
    fs::write(registry_path, toml::to_string(&registry).unwrap()).unwrap();
}

#[test]
fn new_language_maps_are_preserved_without_generator_changes() {
    let root = fixture();
    change_ui(root.path(), |ui| {
        ui["parameters"]["BLURSCALEX"]["label"]["ja"] = serde_json::json!("水平ぼかし");
        ui["parameters"]["BLURSCALEX"]["description"]["ja"] =
            serde_json::json!("水平方向のぼかしを調整します。");
    });
    let out = build_fixture(root.path());
    let ui: serde_json::Value =
        serde_json::from_slice(&fs::read(out.path().join("zfast-crt/ui.json")).unwrap()).unwrap();
    assert_eq!(ui["parameters"]["BLURSCALEX"]["label"]["ja"], "水平ぼかし");
    assert_eq!(ui["default_locale"], "en");
    assert_eq!(ui["languages"], serde_json::json!(["en", "ja", "zh-CN"]));
    // ja 只提供局部翻译；默认语言保证缺失文案存在可回退目标。
    let basic_label = ui["groups"]["basic"]["label"].as_object().unwrap();
    assert!(!basic_label.contains_key("ja"));
    assert!(basic_label.contains_key(ui["default_locale"].as_str().unwrap()));
}

#[test]
fn ui_rejects_unknown_parameter_group_and_missing_default_locale() {
    for case in [
        "unknown_parameter",
        "unknown_group",
        "missing_default_locale",
        "missing_default_label",
    ] {
        let root = fixture();
        change_ui(root.path(), |ui| match case {
            "unknown_parameter" => {
                ui["parameters"]["unknown"] = ui["parameters"]["BLURSCALEX"].clone();
            }
            "unknown_group" => {
                ui["parameters"]["BLURSCALEX"]["group"] = serde_json::json!("missing")
            }
            "missing_default_locale" => {
                ui.as_object_mut().unwrap().remove("default_locale");
            }
            _ => {
                ui["parameters"]["BLURSCALEX"]["label"]
                    .as_object_mut()
                    .unwrap()
                    .remove("en");
            }
        });
        let out = tempfile::tempdir().unwrap();
        let err = continue_shader_carts::build(root.path(), out.path()).unwrap_err();
        assert!(err.to_string().contains("UI"), "{case}: {err}");
    }
}
