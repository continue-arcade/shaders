# Shader Cart 首个联动切片实施计划

日期：2026-10-29。当前目标：建立独立 Shader 仓库，生成一张自包含家族 Cart，并通过 GitHub Release 与现有本地 Cart Station 完成真实分发冒烟。最终 25 个精选的运行/视觉/性能验收仍为后续完整交付范围。

**架构**：本仓库拥有家族登记、固定来源、参数与多语言描述、确定性 Cart 生成。Cart Station 拥有 GitHub 来源发现、验包、签名索引和下载；Core/Dash/Forge 的真实加载与动态 UI 按原有工程阶段接入。

**已确定输入**：`libretro/slang-shaders` commit `2b0530bac0e8083d9d643c194929dc0a6b69bd35`。首张 Cart 为 Zfast CRT 家族，标准入口 `crt/zfast-crt.slangp`，复合视频入口 `crt/zfast-crt-composite.slangp`。其源码与公共 include 明确采用 GPL-2.0-or-later。

## 任务 1：Rust 生成器与首张家族 Cart

文件边界：`Cargo.toml`、`src/{lib,main}.rs` 与按职责拆分的模块、`tests/`、`registry/zfast-crt.toml`、`upstream/`、`LICENSES/`。参数 UI 字段使用统一语言映射。

- [x] 先编写使用实际 fixture 的失败测试：两个变体完整解析、include 参数被提取、入口默认覆盖正确；丢失依赖和包外路径明确失败。
- [x] 固定并核对来源文件摘要，按原相对布局复制至生成 Cart；拒绝未登记或篡改输入，不通过网络在编译过程中隐式取文件。
- [x] 用固定版本 `librashader-presets` 与 `librashader-preprocess` 生成各变体技术描述；合并相同参数 ID，保留来源并报告冲突。
- [x] 生成 `continue.toml`、`cartfile.toml`、参数与 UI JSON、来源文件清单及许可；英文与中文映射完整。
- [x] CLI 固定为 `continue-shader-carts build --family zfast-crt --out <目录>`；输出 `.cart` 和 JSON 构建报告。CLI 文档说明此命令仅生成，不承诺渲染通过。
- [x] ZIP 按固定排序、时间戳和元数据生成；相同输入生成同一摘要。资源入口与所有渲染文件位于包内，只有包内共享。
- [x] 测试覆盖确定性、篡改/缺失/越界拒绝、多语言映射和原始参数 ID；用真实 `continue-cart verify` 复核格式互通。

完成条件：Rust 门禁通过，可重复生成同字节 Cart，正式 Cart 工具能够验包并解包。

## 任务 2：现有 Station 支持家族前缀 tag

工作区：`/Users/karma/krtek-v2`，仅限 `apps/cart-station` 和相应架构说明，保留其他脏改动。

- [x] 先测试证明 `zfast-crt/v0.1.0-smoke.1` 在当前来源规则下被跳过。
- [x] 增加可选 `tag_prefix`，精确匹配前缀后解析 `v?SemVer`；未设置时保留原规则。
- [x] 对其他家族 tag 跳过下载，版本匹配使用去前缀版本，README 和 provenance 保留完整 tag。
- [x] 验证配置、真实 refresh、幂等和默认来源回归，更新来源示例。

完成条件：现有 Station 测试通过，新的家族 tag 不改变旧来源与发布完整性规则。

## 任务 3：GitHub 发布与真实联动冒烟

- [x] 本地完成并复核代码，再在 `continue-arcade/shaders` 建立仓库；仓库仅含本任务代码与许可明确的 Shader 来源，不复制私有产品源码或凭据。
- [x] 增加 CI 门禁及手动生成发布候选的工作流；正式发布由维护者选择。首个真实制品明确标为冒烟 prerelease。
- [x] 本机生成 Cart，并验证 GitHub Release 下载的字节摘要与本地产物一致。
- [x] 为已存在的本地 Cart Station 配置该 Shader 来源，使用明确的 `tag_prefix` 和 prerelease 选择；本机签名密钥保持原样。
- [x] 执行真实 refresh，通过 HTTP 获取 Cart、版本、签名索引和下载；验证第二次刷新不重复导入。
- [x] 用正式客户端验签、验包并在隔离 ContinueSpace 验证安装登记，避免修改真实游戏资料。
- [x] 写入可重放的命令、来源 commit、资源/制品身份、摘要和分发证据，明确没有完成真实 GPU 渲染及 25 项验收。

完成条件：GitHub → 本地 Station → 下载/验证/隔离安装链路有实际结果，报告能区分构建、分发和渲染能力。

## 审阅与交付

- [x] 对新仓库与 Station 适配分别做独立代码复审，修复重要问题后再执行发布冒烟。
- [x] 仅提交本仓库文件；根产品仓库的同期改动不得整体提交。
- [x] 保存剩余 25 项精选、实际编译/渲染、UI 与配置接入的未完成项，不能以冒烟代替最终验收。
