# Continue Shader Carts

Continue Arcade 的 Shader 来源登记、参数元数据和自包含 Cart 生成器。

每个家族一张 Cart。一个 Cart 可以包含多个 Shader 入口、变体与观感，包内可以共享代码；不同 Cart 各自携带完整实现，不在运行时读取另一个 Cart 的内部 pass、include 或纹理。

## 当前交付

当前登记三个独立家族，固定上游均为 `libretro/slang-shaders@2b0530bac0e8083d9d643c194929dc0a6b69bd35`。构建只读本仓库已登记的源文件，不联网补文件。

| 家族 | Cart ID / Shader Resource ID | 版本与 tag | 入口与参数 |
| --- | --- | --- | --- |
| Zfast CRT | `org.continue.shaders.zfast-crt` / `org.continue.shader.zfast-crt` | `0.1.0-smoke.1`；`zfast-crt/v0.1.0-smoke.1` | standard 6；composite 10 |
| Sharp Bilinear | `org.continue.shaders.sharp-bilinear` / `org.continue.shader.sharp-bilinear` | `0.1.0-smoke.1`；`sharp-bilinear/v0.1.0-smoke.1` | standard 2，像素缩放 |
| Dot | `org.continue.shaders.dot` / `org.continue.shader.dot` | `0.1.0-smoke.1`；`dot/v0.1.0-smoke.1` | standard 4，点阵显示 |

冒烟版本用于验证生成、GitHub 分发和 Cart Station 联动。它不表示实际 GPU 渲染、跨平台运行、完整参数 UI 或最终 25 项精选验收已经通过。

## 构建

使用 Rust 1.93.1 与 mise：

```sh
mise trust
mise install
mise run check
mise exec -- cargo run --locked -- families
mise exec -- cargo run --locked -- build --family sharp-bilinear --out dist
mise exec -- cargo run --locked -- build --family dot --out dist
mise exec -- cargo run --locked -- build --family zfast-crt --out dist
```

每个家族输出 `dist/<family>-<version>.cart`、`dist/<family>-report.json` 和可审查的展开树 `dist/<family>/`。可以在同一输出目录依次构建不同家族，已有候选、报告或树均拒绝覆盖。同输入应生成同字节 Cart；参数 ID 保持上游原值，展示文本使用语言标签到文字的映射。

源码与 preset 的技术定义自动提取；按变体输出参数初值、入口默认值、范围与步长。中文/英文名称、说明及分组随 Cart 分发，不需要修改客户端硬编码文案。

## 发布候选

CI 检查固定依赖与源码摘要、真实 preset/include 解析、多语言数据和可复现性。`发布候选` 工作流手动传入 `family`，上传该家族 `.cart`、机器构建报告和 `docs/releases/<family>-<version>.md` 更新说明到 **draft prerelease**；维护者验收后再选择公开发布。不会因上游存在新提交就自动正式发布。候选脚本先校验家族、登记与报告一致性、制品摘要、源码 revision，并检查准确 tag 与 Release 均不存在；网络或鉴权失败不会当成“未发布”。已有发布永不覆盖。

CI 使用 `families` 从 `registry/*.toml` 发现家族。新增家族只增加登记、来源文件、UI、许可记录、家族说明及版本更新说明，不修改 Rust 家族分支或工作流列表。每个家族独立维护 `version`、`tag_prefix`，一次候选只包含一张 Cart；任何消费端不得依赖仓库全局 Latest Release。

登记严格绑定 `org.continue.shaders.<family>`、`org.continue.shader.<family>` 与 `<family>/` tag 前缀。来源根位于 `upstream/`，每个输入固定 SHA-256 与 40 位上游 commit。当前闭包 profile 支持直接 preset、pass 与普通 include；拒绝 `#reference`、外部纹理、optional include、目录穿越、符号链接及未引用文件。超出此 profile 的家族需先扩展并验证闭包能力。参数和多语言结构沿用现行契约，不为每个新家族新增客户端文案分支。

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

本机联动冒烟通过现有 `continue-core` 正式 API 使用隔离 ContinueSpace，不读写用户的真实游戏资料。runner 从 `--expected-cart` 自动取得家族身份、版本和原始参数定义，并通过 `shaders.get` / `shaders.installed.list` 核对资源、范围、默认值和 BCP47 映射。详见 `scripts/smoke_station.py --help` 与后续执行记录。

首轮真实结果见[2026-10-29 联动冒烟记录](docs/verification/2026-10-29-station-smoke.md)：GitHub 候选、跨系统同字节、Station 刷新与正式 Core 安装/卸载均已通过；运行时渲染仍待验收。

## 许可与来源

生成器自有代码采用仓库 `LICENSE` 中的许可。上游 Shader 的许可独立记录，Zfast CRT 为 **GPL-2.0-or-later**；保留 Greg Hogan（SoltanGris42）、metallic77 等原始署名与声明。GPL 许可全文及每份固定文件的来源/摘要随交付保存。包装为 Cart 不改变上游许可。

Sharp Bilinear 与 Dot 的 shader 均在固定源码中明确声明 `Author: Themaister`、`License: Public domain`。登记为 `LicenseRef-Public-Domain`，随包保留来源原文与单独声明记录；不擅自替换为 CC0。两份 preset 无独立许可头，作为同家族配置原样保存并披露来源；不宣称存在额外上游许可文本。每份 shader/preset 字节均另与固定 Git tree 的 blob SHA-1 身份核对。

Sharp Bilinear 的 simple 入口无参数，现行客户端要求每个变体至少一个参数，因此本轮只交付标准入口。LCD Grid v2 也曾考察，但文件缺少就地许可声明，未纳入这次许可清晰的冒烟集合。

## 后续范围

最终目标仍为 25 个精选的明确入口/变体与逐项验收。当前冒烟不缩减该范围。预设依赖语言、自动升级、任意 Shader 叠加和 HDR/子帧运行能力分别按产品与 Dash 计划推进。
