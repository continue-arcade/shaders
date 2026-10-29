# Zfast CRT 0.1.0-smoke.1

这是 Continue Shader Cart 的首个分发联动冒烟版本，标为 prerelease。发布用于验证 GitHub Release → 本地 Cart Station → 正式 Core API 的链路，不是正式的 Shader 运行支持声明。

- Cart：`org.continue.shaders.zfast-crt`。
- 资源：`org.continue.shader.zfast-crt`。
- 变体：`standard`、`composite`，分别提取 6 个与 10 个参数。
- 参数技术描述保留原 ID 和逐变体默认值；UI 文案来自声明式多语言映射。
- 来源：`libretro/slang-shaders@2b0530bac0e8083d9d643c194929dc0a6b69bd35`，5 份固定文件，GPL-2.0-or-later。
- Cart 包含完整来源、包内 include、参数数据、UI 映射、逐文件摘要与许可证。
- Bundle SHA-256：`994f652527d6f7a7b05d74cd15a5aae32e1ee1cc7e3284b1888a88f1216fb3cf`。

生成器已通过格式、编译、Clippy、15 项测试和 rustdoc；输出树通过固定 librashader 的 preset 解析与 include 预处理，并经现行 Continue Cart 工具验包。

本版本未完成 Shader 编译/GPU 渲染、视觉/性能和跨平台运行验收，也未完成最终 25 个精选验收。Station 验包、签名索引与隔离空间安装的结果在独立冒烟报告中记录。
