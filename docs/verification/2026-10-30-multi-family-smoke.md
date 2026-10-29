# Shader Cart 多家族构建、独立发布与分发冒烟

日期：2026-10-30（Asia/Shanghai）。实施 revision：`b8b854b500f15116369da876661bcd95b0f03d66`。

本轮完成登记驱动的多家族生成器和独立候选发布，新增 Sharp Bilinear 与 Dot 两个家族；两者均完成生成、GitHub 分发、签名安装和正式 Shader API 资源参数读取。未进行这两个新家族的 GPU、画面对照、性能或跨平台运行验收，25 项精选目标继续保留。

机器汇总见 [multi-family-summary.json](2026-10-30-multi-family-summary.json)，上游逐文件身份见 [source-blobs.json](2026-10-30-source-blobs.json)。本机原始执行证据位于 `/Volumes/GM9000/continue-shader-families-20261030/`；其中包含隔离空间、JSON-RPC transcript、下载包、发布前后快照及 Station 刷新报告，不将本地私钥、凭据数据库或空间日志上传公共仓库。

## 家族与发布

| 家族 | 效果类别 | Cart ID | Resource ID | 版本与 Release | 参数 |
| --- | --- | --- | --- | --- | --- |
| Sharp Bilinear | 像素缩放 | `org.continue.shaders.sharp-bilinear` | `org.continue.shader.sharp-bilinear` | [sharp-bilinear/v0.1.0-smoke.1](https://github.com/continue-arcade/shaders/releases/tag/sharp-bilinear/v0.1.0-smoke.1) | standard：2 |
| Dot | 点阵显示 | `org.continue.shaders.dot` | `org.continue.shader.dot` | [dot/v0.1.0-smoke.1](https://github.com/continue-arcade/shaders/releases/tag/dot/v0.1.0-smoke.1) | standard：4 |
| Zfast CRT（回归） | CRT | `org.continue.shaders.zfast-crt` | `org.continue.shader.zfast-crt` | [既有 zfast-crt/v0.1.0-smoke.1](https://github.com/continue-arcade/shaders/releases/tag/zfast-crt/v0.1.0-smoke.1) | standard：6；composite：10 |

两个新 Release 均明确为 prerelease，未晋升稳定版，也不依赖仓库全局 Latest。每份 Release 准确附上本家族 Cart、机器构建报告和版本更新说明。工作流只创建 draft prerelease；下载、核对并完成发布前正式 Core 冒烟后，依据本轮直接发布授权依次公开。

| 家族 | Cart 字节数 | SHA-256 |
| --- | ---: | --- |
| Sharp Bilinear | 11,434 | `a34aba5679e3d43629edb3424d8ad2d1faff901cb138d7cf6dcf44367786c278` |
| Dot | 14,523 | `9046354bc58d646f12915395faad8433754fa71a6ab9d34c64c8956a2d01e074` |
| Zfast CRT | 55,279 | `994f652527d6f7a7b05d74cd15a5aae32e1ee1cc7e3284b1888a88f1216fb3cf` |

## 来源与许可

均固定到 `libretro/slang-shaders@2b0530bac0e8083d9d643c194929dc0a6b69bd35`。新家族各自只有一份 preset 和一份单 pass Shader，不含外部纹理或跨 Cart 引用。下载的四份文件先按 Git blob 对象算法核对固定 Git tree 中的 SHA-1 身份，再登记逐文件 SHA-256。保留原始换行和尾空白，不为通过 whitespace 检查改写上游字节。

两份 Shader 原文均明确写有 `Author: Themaister`、`License: Public domain`。许可登记为 `LicenseRef-Public-Domain`，原始声明和来源记录随包分发，不替换为 CC0。preset 没有单独许可头，作为同家族配置原样保留并明确披露，未虚构额外许可文本。生成器仍为 MIT。

Sharp Bilinear 采用 `SHARP_BILINEAR_PRE_SCALE` 和 `AUTO_PRESCALE` 两个上游参数；Dot 保留 `gamma`、`shine`、`blend`、`soft` 四个原始 ID。UI 名称、说明、分组及变体文案均使用 `en` / `zh-CN` 映射，不将语言标签加入参数身份。

曾考察但未交付的入口：

- Sharp Bilinear simple：上游没有参数，当前正式客户端在 `continue-application/src/shaders.rs` 明确拒绝空参数变体。本轮只交付同家族标准入口，未改并行中的共享契约；没有把无参数入口伪造成可用。
- LCD Grid v2：选定源码缺少就地许可声明，未作为本次许可清晰的冒烟集合。Dot 的声明明确，因此选用 Dot 完成第二家族。

两个实际选定家族都完成了端到端读取，无第二家族阻断项。

## 生成器与工作流

`registry/<family>.toml` 驱动身份、独立版本、来源根、固定 commit、逐文件摘要、许可、包元信息、变体、UI 文件、tag 前缀与当前版本更新说明。`families` 命令从登记目录发现家族；CLI、生成器和 CI 不再对家族添加分支。

严格校验包括：家族 slug、SemVer、40 位 commit、来源 URL、Cart/Resource ID 与家族的绑定、准确 `<family>/` 前缀、版本说明路径、规范相对路径、符号链接拒绝、SHA-256、唯一变体、保留名称及生成元数据路径、UI 关联和 BCP47 标签。`family` 不能作为变体 ID，避免覆盖 `parameters/family.json`。路径插入 TOML 时使用 TOML 字符串编码。

闭包 profile 当前支持直接 preset、pass 和普通 include，拒绝 `#reference`、纹理、optional include 及 preset 块注释。preset 在交给 librashader 前严格检查 key/value 类型，防止上游将未知字符串键推断为纹理，或吞掉块注释后读取隐藏的外部 preset。闭包从已验证的内存字节计算，真实 librashader 解析只读取这些字节组成的临时快照，避免校验后从原来源目录重读。

候选 workflow 使用环境变量和 argv 传递 family，不把用户输入直接插入 shell 命令。候选脚本校验登记、报告、准确制品摘要、源码 revision 与当前家族说明，并检查准确 tag 和 Release 均不存在；网络或鉴权失败不会当作不存在。每次只发布一个家族，不覆盖已有制品。

## 验证阶段与证据

| 阶段 | 结果与边界 |
| --- | --- |
| 本地工程门禁 | `CARGO_TARGET_DIR=/tmp/continue-shader-families-target RUSTDOCFLAGS='-D warnings' mise run check` 通过：fmt、check、严格 Clippy、26 项 Rust 集成测试、严格 Rustdoc、6 项 Python 测试 |
| Ubuntu 全家族 CI | [40922205590](https://github.com/continue-arcade/shaders/actions/runs/40922205590) 通过，按登记发现并构建三个家族 |
| Sharp Bilinear 候选 | [40922218607](https://github.com/continue-arcade/shaders/actions/runs/40922218607) 通过 |
| Dot 候选 | [40922226755](https://github.com/continue-arcade/shaders/actions/runs/40922226755) 通过 |
| 确定性 | 同输入双构建同字节；两个 Ubuntu 候选分别与 macOS 本地产物逐字节一致；Zfast 仍匹配历史已发布 SHA-256 |
| 来源、许可、闭包 | 逐文件摘要、上游 Git blob、原始许可声明、完整 provenance；展开树可独立被真实 librashader 解析/预处理 |
| Cart 格式 | 正式 `continue-cart verify` 三家族退出 0；通用单包签名功能仍提示 local/untrusted，不能将其当作签名验证 |
| 发布前安装 | 独立本地导入 Station `14320`，两个新家族通过正式 Core 签名索引安装、读取与卸载保护检查 |
| GitHub 分发 | 独立 GitHub 来源 Station `14319`，依次完成 Zfast 基线、Sharp 发布后、Dot 发布后的真实 refresh；准确前缀筛选、每家族 Cart ID/版本校验与 HTTP 摘要均通过 |
| 安装与资源读取 | 三家族分别在全新空间通过正式 Core 签名安装、`shaders.get`、`shaders.installed.list`；原参数 ID、初值、默认值、范围、步长与语言映射均匹配实际 Cart |
| 同空间共存 | 同一隔离空间安装三家族，列表无问题；移除 Dot 后另外两家族的版本/参数不变；重装 Dot 后三家族恢复共存，34 份不可变文件摘要全部不变 |
| 实际渲染 | 本轮未请求 `shaders.preview.get`，未验证新家族的 GPU 编译/渲染、视觉或性能 |
| 跨平台与最终精选 | Ubuntu/macOS 生成字节一致；没有据此宣称 Windows/Linux/macOS 运行渲染或全部 25 项精选通过 |

Core、Cart 工具与 Station 使用现有已构建程序的独立副本，放在本轮独有目录；未重建其他 session 正在执行的主仓库二进制。用户、设备配置根与 ContinueSpace 都位于隔离测试输出下。

独立复审发现并修复了隐式纹理、块注释隐藏 `#reference`、变体汇总名称碰撞和入口字符串编码问题，均有失败回归/实际触发验证。复核后未留下实质性问题。

## 家族独立性与旧制品保护

1. 集成测试分别将 Dot 和 Sharp Bilinear 从 `0.1.0-smoke.1` 改为 `0.1.0-smoke.2`：只改变当前家族，其他家族版本与 Cart 字节完全相同；测试后恢复输入，没有发布 smoke.2。
2. Sharp Bilinear 公开前后对照 GitHub 快照：Dot 草稿的版本、状态与 Asset ID/摘要/大小不变，Zfast 同样不变。
3. Dot 公开前后：Sharp Bilinear 与 Zfast 的状态、版本、Asset ID/摘要/大小不变。
4. 实际调用候选脚本尝试创建既有 Zfast tag，退出 1 并明确拒绝覆盖，未进入 Release 创建。
5. 再次下载历史 Zfast Cart，仍为 55,279 字节和历史摘要；未重建、上传或替换其 Release 资产。

## 共享 4318 Station 与限流情况

主仓库只对 `apps/cart-station/station.toml` 追加 Sharp Bilinear 与 Dot 两项来源，保留原文件全部前缀内容和 Zfast 配置。没有将主仓库其他脏工作区修改暂存或提交。

共享 Station 的一次性 GitHub refresh 在读取原 Zfast README 时收到匿名限流 HTTP 403，报告要求至少等待 1761 秒；后两个新来源因此未请求。隔离 Station 之后补做的全量重复 refresh 同样受限流影响。本轮没有将这两次尝试记为成功，也未修改 Station 的鉴权、重试或主线代码。

为使现有 4318 立即提供这两个家族，通过既有 `import` 命令收存已从 Ubuntu GitHub Release 下载并与本机逐字节核对的 Cart。随后通过正在运行的 4318 HTTP 读取三个家族版本与下载，摘要全部正确；Station 公钥及 key ID 与操作前完全一致。未重启或打断现有服务，未向用户真实空间安装资源。

因此，共享服务已能分发新家族；共享配置的 GitHub 自动来源刷新仍待限流解除后成功执行。现有运行进程不热读配置文件，两项来源已落盘供下次正常加载，本轮一次性刷新尝试及本地导入均使用新配置。共享服务新制品的本轮导入来源为本地导入；隔离 Station 的对应制品保留真实 GitHub 来源证据，不能混写为共享 GitHub refresh 已成功。

两个临时 Station 服务 `14319` / `14320` 和所有测试 Core 进程已关闭；隔离数据与报告保留供复查。尚未验证的项目为新家族实际渲染、视觉/性能、跨平台运行、无参数 simple 入口兼容性与全部 25 项精选；这些不因分发安装通过而改变状态。
