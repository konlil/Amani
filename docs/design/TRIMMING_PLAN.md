# LLM3dEngine Godot 裁剪计划

## 裁剪策略

不修改 Godot 源码，不删除代码。通过编译参数控制模块的启用/禁用，保持 submodule 干净。自定义模块（QuickJS 脚本层、协调层）放在 Godot 源码树之外，通过 `custom_modules` 参数引入。

## 编译验证基线

原始 Godot 4.4.1 macOS editor 构建已验证通过（arm64，耗时约 5 分钟）。

---

## 构建配置

### 裁剪构建命令

```bash
scons platform=macos target=template_release \
  vulkan_sdk_path=/opt/homebrew/opt/molten-vk \
  tools=no \
  -j$(sysctl -n hw.ncpu) \
  \
  module_visual_script_enabled=no \
  module_mono_enabled=no \
  module_mobile_vr_enabled=no \
  module_openxr_enabled=no \
  module_webxr_enabled=no \
  module_lightmapper_rd_enabled=no \
  module_raycast_enabled=no \
  module_theora_enabled=no \
  module_webrtc_enabled=no \
  module_cvtt_enabled=no \
  module_betsy_enabled=no \
  module_gridmap_enabled=no \
  module_csg_enabled=no \
  \
  custom_modules=../../modules
```

### 禁用模块清单

| 模块 | 禁用理由 |
|------|----------|
| visual_script | 已废弃 |
| mono | 不需要 C# 支持 |
| mobile_vr | 不需要 VR |
| openxr | 不需要 VR/AR |
| webxr | 不需要 WebXR |
| lightmapper_rd | 编辑器专用光照烘焙 |
| raycast | Embree 光线追踪，编辑器烘焙用 |
| theora | 视频编解码，核心不需要 |
| webrtc | 用 ENet/WebSocket 足够 |
| cvtt | 纹理压缩，编辑器导入用 |
| betsy | 纹理压缩，编辑器导入用 |
| gridmap | 3D 网格地图编辑器工具 |
| csg | CSG 建模工具 |

### 保留模块清单

| 模块 | 保留理由 |
|------|----------|
| gdscript | 引擎内部胶水代码 |
| navigation | NPC 寻路 |
| godot_physics_3d | 3D 物理 |
| godot_physics_2d | 2D 物理（UI 等可能依赖） |
| jolt_physics | 可选高性能物理后端 |
| multiplayer | 多人游戏框架 |
| enet | 网络传输 |
| websocket | WebSocket 网络 |
| gltf | glTF 模型运行时加载 |
| freetype | 字体渲染 |
| text_server_adv | 文本渲染 |
| msdfgen | SDF 字体 |
| ogg / vorbis / minimp3 | 音频格式 |
| noise | 程序化生成 |
| regex | 正则表达式 |
| basis_universal | 纹理运行时解压 |
| png / jpg / bmp / tga / webp / hdr / tinyexr / dds / ktx | 图片格式 |
| svg | SVG 渲染 |
| astcenc / etcpak / bcdec | 纹理解压 |

---

## 项目目录结构

```
LLM3dEnine/
├── engine/
│   └── godot/                  ← Godot 4.4.1 submodule（不修改）
├── modules/                    ← 自定义模块（通过 custom_modules 引入）
│   ├── quickjs_scripting/      ← QuickJS 脚本运行时
│   └── llm_coordinator/        ← LLM 协调层
├── tools/
│   └── binding_generator/      ← Binding 代码生成器
├── build/
│   └── macos.sh                ← macOS 构建脚本（封装 scons 参数）
├── docs/
│   └── design/
└── README.md
```

关键点：`custom_modules` 参数让 Godot 构建系统扫描外部目录中的模块，与内置模块一样参与编译。自定义模块放在 `LLM3dEnine/modules/` 下，与 Godot 源码完全分离。

---

## 自定义模块：quickjs_scripting

```
modules/quickjs_scripting/
├── SCsub                       ← 构建脚本
├── config.py                   ← 模块配置
├── register_types.h
├── register_types.cpp
├── quickjs_runtime.h           ← QuickJS 运行时封装
├── quickjs_runtime.cpp
├── quickjs_binding_gen.h       ← 自动生成的 binding
├── quickjs_binding_gen.cpp
└── thirdparty/
    └── quickjs/                ← QuickJS 源码
```

集成点（在 Godot 主循环中）：
- `_initialize()` — 启动时初始化 QuickJS，加载入口 JS
- `_process(delta)` — 每帧调用 JS 回调
- `_physics_process(delta)` — 每物理帧调用 JS 回调
- `_finalize()` — 关闭时清理

---

## 自定义模块：llm_coordinator

```
modules/llm_coordinator/
├── SCsub
├── config.py
├── register_types.h
├── register_types.cpp
├── coordinator.h               ← 协调层主类
├── coordinator.cpp
├── ts_compiler.h               ← tsc 编译调用
├── ts_compiler.cpp
├── linter.h                    ← ESLint 检查调用
├── linter.cpp
├── state_collector.h           ← 场景状态收集（JSON）
├── state_collector.cpp
├── screenshot.h                ← 截屏
└── screenshot.cpp
```

---

## Binding 代码生成器

```
tools/binding_generator/
├── generate.py                 ← 主生成脚本
├── templates/
│   ├── binding.cpp.j2          ← C++ binding 模板
│   └── types.d.ts.j2           ← TypeScript 声明模板
└── config/
    ├── core_api.json           ← 核心 API 白名单
    └── extended_api.json       ← 扩展 API 白名单
```

输入源：Godot 构建时生成的 `extension_api.json`（包含所有类、方法、属性、信号的完整描述）。

生成产物：
- `quickjs_binding_gen.cpp` → 编译进引擎
- `engine-core.d.ts` → LLM 查阅的核心 API 声明
- `engine-extended.d.ts` → 扩展 API 声明

---

## 执行顺序

| 步骤 | 内容 | 依赖 |
|------|------|------|
| 1 | 编写 macOS 构建脚本，用裁剪参数编译，验证通过 | 无 |
| 2 | 创建 quickjs_scripting 模块骨架，验证 custom_modules 能被构建系统识别 | 步骤 1 |
| 3 | 嵌入 QuickJS 源码，实现基础 JS 执行能力 | 步骤 2 |
| 4 | 编写 binding 生成器，生成核心 API 的 binding | 步骤 3 |
| 5 | 创建 llm_coordinator 模块，实现 tsc 编译 + ESLint 检查 + 状态收集 | 步骤 3 |
| 6 | 端到端验证：LLM 写 TS → 编译 → 执行 → 反馈 | 步骤 4, 5 |

---

## 下一步行动

1. 编写 `build/macos.sh` 构建脚本
2. 执行裁剪构建，验证通过
3. 记录裁剪前后产物大小对比
