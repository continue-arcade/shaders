# Zfast CRT

一组轻量显像管效果，提供标准扫描线/遮罩和复合视频变体。适合希望获得基础 CRT 外观的像素游戏；实际设备性能和视觉表现仍需验收。

| 变体 | 上游入口 | 说明 |
| --- | --- | --- |
| `standard` | `crt/zfast-crt.slangp` | 标准细遮罩与扫描线，公共实现通过包内 include 共享 |
| `composite` | `crt/zfast-crt-composite.slangp` | 增加复合视频色彩混合、曲率等调整项 |

参数采用上游 ID 和技术范围；展示名、说明和分组使用 `en` / `zh-CN` 语言映射，用户配置始终保存原参数 ID 与数字。

当前 `0.1.0-smoke.1` 仅验证生成、验包与分发联动，不是 GPU/跨平台验收结果。两个变体的源码、所需 include、来源摘要和许可都在同一个 Cart 中，运行时不读取其他 Cart。

上游：`libretro/slang-shaders` commit `2b0530bac0e8083d9d643c194929dc0a6b69bd35`。作者：Greg Hogan（SoltanGris42）；复合视频等修改署名见原文件。许可：GPL-2.0-or-later。
