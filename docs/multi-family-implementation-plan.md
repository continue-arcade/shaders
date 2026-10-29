# Shader Cart 多家族执行计划

2026-10-30。按本轮已确认的架构和直接实施授权执行。

目标：登记驱动构建与家族独立发布，新增 Sharp Bilinear（像素缩放）和 Dot（点阵显示）两个轻量自包含家族。保持现行 Cart、参数、UI 契约；Zfast 原版本整包摘要必须保持不变。

- [x] 在现有集成测试加入多家族、恶意输入、摘要不变与独立升版隔离的失败断言。
- [x] 登记驱动身份、来源根、许可、包文案、tag 与更新说明；严格检查家族 slug、SemVer、固定 commit、路径、唯一 ID 与摘要；CLI 不含家族分支。
- [x] 固定上游文件与 Git blob/SHA-256；英中映射使用原始参数 ID；独立包内完整闭包经真实 librashader 解析。
- [x] CI 由登记发现所有家族；候选工作流只接收家族输入，经登记生成准确 tag、文件与更新说明，拒绝已有 tag/Release，不覆盖制品。
- [x] 运行完整 Rust 门禁与 Python 故障测试；双构建比较所有家族，验证单家族升版不改其他家族。
- [x] 提交并推送本仓库本任务变更，逐家族运行候选工作流、下载对照，公开明确的 smoke prerelease。
- [x] 独立 Station 端口/数据与 ContinueSpace，经真实 GitHub refresh、签名安装与正式 Shader API 读取资源参数；不调用预览、不作 GPU/跨平台声明。
- [x] 保存中文证据和未验范围；仅按必要最小增量补共享 Station 来源，不操作现有服务。

范围：不改主仓库共享 Shader DTO、客户端 UI、设置、Metal 或 Dash。25 项精选目标保持。包内历史文字及旧制品不可变；公共仓库不保存凭据、本地密钥与隔离空间日志。

执行补充：独立复审发现 preset 宽容解析可绕过逐行闭包检查，已用失败回归验证并修复块注释隐藏 #reference、隐式纹理、保留元数据碰撞；上游解析只接触 SHA 校验后建立的隔离快照。Zfast 原 Cart 摘要维持不变。

结果：两个家族均完整交付，见 `docs/verification/2026-10-30-multi-family-smoke.md`。共享 Station 的 GitHub 刷新遇到匿名限流；已通过校验过的 Release 资产本地导入并确认 4318 立即分发，真实 GitHub refresh 成功证据来自隔离 Station。GPU 与跨平台运行未验证。
