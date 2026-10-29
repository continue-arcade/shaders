# Zfast CRT：GitHub 与本地 Cart Station 联动冒烟

日期：2026-10-29（Asia/Shanghai）。结果：构建与真实分发联动通过。真实 GPU、视觉、性能和 25 项精选最终验收尚未完成。

## 已发布内容

- 仓库：[continue-arcade/shaders](https://github.com/continue-arcade/shaders)。
- 源码与候选构建 revision：`d60f546b5804f5e5bb89792c3e4469d14aaf93ba`。
- 制品：[zfast-crt/v0.1.0-smoke.1](https://github.com/continue-arcade/shaders/releases/tag/zfast-crt/v0.1.0-smoke.1)，明确标为 prerelease。
- Cart ID：`org.continue.shaders.zfast-crt`。
- Shader Resource ID：`org.continue.shader.zfast-crt`。
- 文件：`zfast-crt-0.1.0-smoke.1.cart`，55,279 字节。
- SHA-256：`994f652527d6f7a7b05d74cd15a5aae32e1ee1cc7e3284b1888a88f1216fb3cf`。
- Station artifact：`artifact.d4b841a04826b0820ea97303354b3cd23e9ce79046a1c2059d9cf4a0dec35290`。

上游固定为 `libretro/slang-shaders@2b0530bac0e8083d9d643c194929dc0a6b69bd35`。5 份来源文件另与官方 Git tree 的 blob 身份逐一核对一致；保留原始 CRLF、署名、许可和内容摘要。

## 构建证据

| 检查 | 结果 |
| --- | --- |
| 本机 Rust fmt/check/Clippy `-D warnings`/rustdoc | 通过 |
| 生成器集成测试 | 15 项通过 |
| 冒烟 runner 故障路径测试 | 2 项通过，覆盖损坏公钥与握手失败进程回收 |
| Ubuntu CI | [40899965609](https://github.com/continue-arcade/shaders/actions/runs/40899965609) 通过 |
| 手动候选工作流 | [40900173624](https://github.com/continue-arcade/shaders/actions/runs/40900173624) 通过，自动建立 draft prerelease |
| 跨构建环境确定性 | 下载 Ubuntu draft asset，与本机 macOS 产物逐字节相同 |
| preset/include | 固定 librashader 0.12.1 真实解析/预处理；标准 6 参数，复合视频 10 参数 |
| 多语言 | 声明式映射与默认语言；新增局部 `ja` 翻译自动进入可用语言集合，默认回退目标仍有效 |
| Continue Cart 格式 | 现行正式 `continue-cart verify` / `unpack` 退出 0 |

首次 Ubuntu CI 因最小 Rust 工具链没有 rustfmt 而失败；随后显式安装固定工具链的 rustfmt/clippy，修复后的 CI 与候选工作流均通过。

候选先由 CI 建立草稿，下载并验证后，按本轮用户授权手动公开为冒烟 prerelease；工作流仍然只生成 draft，不自动正式发布。

## 本地 Station

使用现有 `org.continue.cart-station.local` 与 `http://127.0.0.1:4318/`，沿用原有数据目录和签名身份；独立取得的公开密钥与 Forge 现有 pin 一致，没有换钥。

新增来源配置：

```toml
[[carts]]
cart_id = "org.continue.shaders.zfast-crt"
github_repo = "continue-arcade/shaders"
tag_prefix = "zfast-crt/"
include_prereleases = true
readme_path = "docs/carts/zfast-crt.md"
readme_ref = "main"
```

Station 来源级 `tag_prefix` 改动在产品仓库，已通过独立复审与 41 项 crate 测试、Clippy 和修改文件格式检查。完整 tag 用于 README 与 provenance，剥离后的版本为 `0.1.0-smoke.1`。未配置前缀的原来源保持原规则。

第一次真实 GitHub refresh 导入 1 份制品，来源无错误；第二次 refresh 零新增并明确跳过已收存 Asset，GitHub asset ID 为 `688966461`。HTTP 目录仍保留原有 Wikidata 与 Pegasus 本地插件 Cart，并新增 Zfast CRT。

## 正式 Core API 隔离空间验证

`scripts/smoke_station.py` 通过实际 `continue-core` JSON-RPC 完成：

1. 独立公钥 DER 与 key_id 校验，HTTP 索引的签名身份匹配。
2. Station HTTP 获取版本与下载，摘要和本地产物一致。
3. `carts.station.open` / `browse` 取得真实版本。
4. `carts.station.install` 与任务轮询完成；正式 Core 校验已 pin 的签名索引、制品摘要并实际解包。
5. `carts.station.status.list` 返回 `stored`，版本与摘要正确，存在真实 `continue.toml`。
6. `carts.station.remove` 移除隔离空间的安装登记，用户测试文件与不可变制品目录均保留。

用户空间、用户配置和设备配置使用独立临时目录；没有向用户真实游戏空间安装或删除资源。

重放方式（需先通过本地 Station 的 `public-key` 命令独立保存公开 JSON）：

```sh
python3 scripts/smoke_station.py \
  --station-url http://127.0.0.1:4318/ \
  --public-key /path/to/station-public-key.json \
  --core /path/to/continue-core \
  --cart-tool /path/to/continue-cart \
  --expected-cart dist/zfast-crt-0.1.0-smoke.1.cart \
  --out smoke-results/new-run
```

`--out` 必须是新目录，详细 JSON-RPC 与日志保留在本机忽略目录，不作为公共制品上传。

## 验证边界

`continue-cart verify` 的通用单包输出仍含“signature verification is not implemented / local-untrusted”；它负责本次格式校验。实际信任证明由正式 Core 安装环节使用独立 pin 的 Station 签名索引完成。本实例仍采用 `other` 资源信任分类，本轮没有提升为官方信任根。

没有完成 Shader 编译成目标 GPU 管线、真实渲染、画面对照、性能、跨平台、Forge 动态参数 UI 或设置持久化，也没有完成 25 项精选。`stored` 表示校验与安装登记，不表示 Shader 已启用或能运行。
