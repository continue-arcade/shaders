# Agent instructions

- 优先使用中文注释、回复和项目文档。对外品牌为 Continue，产品名为 Continue Arcade。
- 本仓库维护 Shader Cart 的来源登记、确定性生成和发布候选。一个家族一张 Cart，Cart 内共享，跨 Cart 各自携带完整实现。
- 使用 Rust；禁止 unsafe 和 dbg!；可恢复错误返回 Result。通过 mise exec -- 运行项目命令。
- 技术参数来自固定版本的 librashader 解析结果。所有展示文案使用以语言标签为键的多语言映射，参数 ID 与配置数值保持语言无关。
- 构建输入固定上游 commit 和文件 SHA-256；打包保留来源、版权与许可证。源码、生成物及日志不保存凭据。
- 新功能先写有意义的失败测试。交付前执行 fmt、check、clippy、test 和 rustdoc；不要把打包或分发冒烟称为真实 Shader 渲染验收。
- 修改范围保持清晰，保存其他任务的未提交改动；不得 force push 或覆盖已经发布的制品。
