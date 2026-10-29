# Continue Shader Carts

Continue Arcade 的 Shader 来源登记、参数元数据和自包含 Cart 生成器。

每个家族一张 Cart。一个 Cart 可以包含多个 Shader 入口、变体与观感，包内可以共享代码；不同 Cart 各自携带完整实现，不在运行时读取另一个 Cart 的内部 pass、include 或纹理。

## 当前交付

首个切片为 **Zfast CRT**，包含标准与复合视频两个变体。上游来源固定为 `libretro/slang-shaders` 的 commit `2b0530bac0e8083d9d643c194929dc0a6b69bd35`，构建从本仓库登记的源文件生成，不在构建期间联网补文件。

| 身份 | 值 |
| --- | --- |
| Cart ID | `org.continue.shaders.zfast-crt` |
| Shader Resource ID | `org.continue.shader.zfast-crt` |
| 初始版本 | `0.1.0-smoke.1` |
| 家族 Release tag | `zfast-crt/v0.1.0-smoke.1` |

冒烟版本用于验证生成、GitHub 分发和 Cart Station 联动。它不表示实际 GPU 渲染、跨平台运行、完整参数 UI 或最终 25 项精选验收已经通过。

## 构建

使用 Rust 1.93.1 与 mise：

```sh
mise trust
mise install
mise run check
mise exec -- cargo run --locked -- build --family zfast-crt --out dist
```

输出包括 `dist/zfast-crt-0.1.0-smoke.1.cart`、`dist/zfast-crt-report.json` 和可审查的展开树 `dist/zfast-crt/`。同输入应生成同字节 Cart；参数 ID 保持上游原值，展示文本使用语言标签到文字的映射。

源码与 preset 的技术定义自动提取；按变体输出参数初值、入口默认值、范围与步长。中文/英文名称、说明及分组随 Cart 分发，不需要修改客户端硬编码文案。

## 发布候选

CI 检查固定依赖与源码摘要、真实 preset/include 解析、多语言数据和可复现性。`发布候选` 工作流手动触发，上传 `.cart` 和报告到 **draft prerelease**；维护者验收后再选择公开发布。不会因上游存在新提交就自动正式发布。

初始冒烟由维护者明确发布为 prerelease，Station 来源需显式设置 `include_prereleases = true`。后续每个家族按自己的 tag 前缀和 CartRelease 独立发布，已经发布的制品不覆盖。

## 与 Cart Station 联动

现有 Cart Station 需要支持来源级 `tag_prefix`：

```toml
[[carts]]
cart_id = "org.continue.shaders.zfast-crt"
github_repo = "continue-arcade/shaders"
tag_prefix = "zfast-crt/"
include_prereleases = true
readme_path = "docs/carts/zfast-crt.md"
readme_ref = "main"
```

Station 剥离准确前缀后按 SemVer 选择版本，README 与来源记录仍使用完整 tag；它负责验证、收存、签名索引和下载，不编译 Shader。

本机联动冒烟通过现有 `continue-core` 正式 API 使用隔离 ContinueSpace，不读写用户的真实游戏资料。详见 `scripts/smoke_station.py --help` 与后续执行记录。

## 许可与来源

生成器自有代码采用仓库 `LICENSE` 中的许可。上游 Shader 的许可独立记录，Zfast CRT 为 **GPL-2.0-or-later**；保留 Greg Hogan（SoltanGris42）、metallic77 等原始署名与声明。GPL 许可全文及每份固定文件的来源/摘要随交付保存。包装为 Cart 不改变上游许可。

## 后续范围

最终目标仍为 25 个精选的明确入口/变体与逐项验收。当前冒烟不缩减该范围。预设依赖语言、自动升级、任意 Shader 叠加和 HDR/子帧运行能力分别按产品与 Dash 计划推进。
