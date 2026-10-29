#![forbid(unsafe_code)]
use std::path::{Path, PathBuf};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("build") {
        return Err("用法: continue-shader-carts build --family zfast-crt --out <目录>".into());
    }
    let mut family = None;
    let mut out = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--family" if family.is_none() => family = args.next(),
            "--out" if out.is_none() => out = args.next().map(PathBuf::from),
            _ => return Err(format!("未知或重复参数: {arg}").into()),
        }
    }
    if family.as_deref() != Some("zfast-crt") {
        return Err("首个切片只支持 --family zfast-crt".into());
    }
    let out = out.ok_or("缺少 --out <目录>")?;
    // CLI 从当前目录读取公开仓库输入，无需私有 Continue 产品仓库。
    let report = continue_shader_carts::build(Path::new("."), &out)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
