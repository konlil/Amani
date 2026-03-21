# LLM3dEngine 引擎基座设计文档

## 概述

本文档描述 LLM3dEngine 引擎核心（C++ 层）的架构设计。引擎基于 Godot Engine（MIT 许可）fork 裁剪，保留渲染/物理/音频等核心运行时能力，去掉人类向的编辑器和脚本系统，接入 TypeScript 脚本层供 LLM 使用。

---

## 基座选择：Godot Engine

### 选择理由

- MIT 许可，无任何商业限制，可自由裁剪和分发
- 代码规模可控（~150 万行 C++），架构可读性好
- 模块化设计，支持编译时裁剪
- 社区活跃，持续演进
- 渲染能力满足当前需求，后续可持续增强

### 裁剪策略

**保留的模块：**

| 模块 | 说明 |
|------|------|
| RenderingServer | Vulkan/OpenGL 渲染管线、材质、Shader、灯光、阴影、粒子 |
| PhysicsServer3D | 3D 物理、碰撞检测、刚体、角色控制器 |
| AudioServer | 音频播放、空间音效 |
| NavigationServer3D | 寻路、导航网格 |
| 动画系统 | 骨骼动画、AnimationPlayer、AnimationTree、状态机 |
| 网络 | MultiplayerPeer、RPC、同步机制 |
| UI 运行时 | Control 节点体系，游戏内 UI 渲染 |
| 场景树 | SceneTree、Node 层级结构、场景实例化 |
| 资源管理 | ResourceLoader、资源缓存、异步加载 |

**去掉的模块：**

| 模块 | 去掉理由 |
|------|----------|
| 编辑器（editor/） | LLM 不需要可视化编辑器 |
| VisualScript | 已废弃，不需要 |
| GDScript 对外暴露 | LLM 使用 TypeScript，不直接接触 GDScript |

**保留但不暴露的模块：**

| 模块 | 说明 |
|------|------|
| GDScript 运行时 | 保留在引擎内部作为胶水代码，不暴露给 LLM |

---

## 场景模型

### 决策：使用 Godot 原生场景树

直接使用 Godot 的 SceneTree + Node 层级结构，不额外包装 ECS 抽象层。

### 理由

- LLM 训练数据中包含大量 Godot 场景树风格的代码，API 模式 LLM 已经熟悉
- 额外包装 ECS 层会引入 LLM 训练数据中不存在的自定义抽象，降低代码生成准确率
- LLM 可以参考现有 Godot 教程和示例来理解 API 用法
- TS binding 层忠实映射 Godot Node 体系，LLM 写出的代码逻辑上与 GDScript 操作场景树一致

### TS 侧 API 示例

```ts
// 创建场景结构
const player = scene.createNode3D("player");

const mesh = player.addChild(MeshInstance3D, "body");
mesh.mesh = assets.load("player.glb");

const camera = player.addChild(Camera3D, "camera");
camera.position = new Vec3(0, 1.8, 0);

const collider = player.addChild(CollisionShape3D, "collider");
collider.shape = new CapsuleShape3D(0.5, 1.8);
```

---

## TypeScript Binding 层

### 接入方式：直接 C++ 层 QuickJS Binding

在 Godot C++ 源码中直接嵌入 QuickJS，通过 C++ 代码将 Godot API 注册到 JS 运行时。不使用 GDExtension 中间层。

### 选择理由

- 既然已经 fork Godot 源码进行裁剪，"需要改源码"不再是额外成本
- 直接调用 Godot C++ API，无中间层，性能最优
- 能访问 Godot 所有内部 API，不受 GDExtension 暴露范围限制
- GDExtension 是为插件设计的，不适合"替代整个脚本系统"这种深度集成

### Binding 生成策略：自动生成 + 分层暴露

**生成侧：**

编写代码生成器，读取 Godot ClassDB（类注册系统），自动输出：

1. C++ 侧 QuickJS binding 代码（函数注册、属性访问器、信号连接）
2. TypeScript 侧 `.d.ts` 类型声明文件

```
Godot ClassDB
    ↓
代码生成器（构建时运行）
    ↓
├── binding_generated.cpp   → QuickJS 函数注册
└── engine.d.ts             → TypeScript 类型声明
```

**暴露侧：`.d.ts` 分层**

```
engine-core.d.ts        ← LLM 默认加载，精选的核心 API
                           Node3D, MeshInstance3D, Camera3D, RigidBody3D,
                           AudioStreamPlayer3D, AnimationPlayer, Control 等

engine-extended.d.ts    ← 高级/冷门 API，LLM 按需引入
                           底层 RenderingServer 直接调用、
                           自定义 Shader 参数、物理射线细节配置等
```

分层标准：
- 核心层：LLM 构建常见游戏类型（FPS、RPG、平台跳跃等）所需的 API
- 扩展层：性能优化、底层定制、特殊效果等高级场景

### 运行时策略

```
开发阶段：QuickJS
    - 轻量（~几百 KB），嵌入简单
    - 快速启动，适合高频编译-运行-反馈循环
    - 解释执行，性能够用（重计算在 C++ 引擎层）

发布阶段：V8（后续接入）
    - JIT 编译，性能比 QuickJS 快 10-100 倍
    - 适合最终发布的游戏产品
```

API binding 层做抽象，底层 JS 运行时可切换：

```cpp
// 运行时抽象接口
class ScriptRuntime {
public:
    virtual ~ScriptRuntime() = default;
    virtual void initialize() = 0;
    virtual void execute(const std::string& js_code) = 0;
    virtual void register_function(const std::string& name, NativeFunction fn) = 0;
    virtual Value call_function(const std::string& name, const std::vector<Value>& args) = 0;
};

class QuickJSRuntime : public ScriptRuntime { /* ... */ };
class V8Runtime : public ScriptRuntime { /* ... */ };  // 后续实现
```

---

## 编译与验证管线

LLM 生成的 TypeScript 代码在执行前经过多层检查：

```
LLM 生成 .ts 文件
    ↓
tsc 编译（类型检查 + 编译为 JS）
    ↓  失败 → 结构化错误反馈给 LLM
ESLint 自定义规则检查（禁止 any/eval/动态 import 等）
    ↓  失败 → 结构化错误反馈给 LLM
QuickJS 执行
    ↓  运行时错误 → 结构化错误反馈给 LLM
引擎运行 → 状态收集 → 反馈给 LLM
```

### TypeScript 约束规则

```jsonc
// tsconfig.json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "noUncheckedIndexedAccess": true,
    "target": "ES2020",
    "module": "ES2020"
  }
}
```

```js
// .eslintrc — 自定义规则（黑名单）
{
  "rules": {
    "no-eval": "error",
    "no-new-func": "error",
    "no-implied-eval": "error",
    "@typescript-eslint/no-explicit-any": "error",
    "no-restricted-syntax": ["error",
      { "selector": "TSTypeReference[typeName.name='Function']" }
    ]
  }
}
```

---

## 数据流总览

```
┌──────────────────────────────────────────────────────────┐
│                       LLM 端                              │
│  接收需求 → 查阅 engine-core.d.ts → 生成/修改 .ts 文件     │
│       ↑                                                   │
│       │ 结构化反馈                                         │
│       │ (编译错误 / 运行时错误 / 场景状态 JSON / 截屏)       │
├───────┼──────────────────────────────────────────────────-─┤
│       │              引擎协调层                             │
│       │                                                   │
│  tsc 编译 → ESLint 检查 → QuickJS 执行                     │
│       │                      ↓                            │
│       └────── 状态收集 ← Godot 运行时                      │
│              (JSON + 截屏 + 人类检查点反馈)                  │
├──────────────────────────────────────────────────────────-─┤
│           Godot 引擎核心 (C++, fork 裁剪)                   │
│                                                           │
│  ┌─────────┐ ┌─────────┐ ┌───────┐ ┌──────┐ ┌─────────┐  │
│  │ 渲染    │ │ 物理    │ │ 音频  │ │ 网络 │ │ 寻路    │  │
│  │ Server  │ │ Server  │ │Server │ │      │ │ Server  │  │
│  └─────────┘ └─────────┘ └───────┘ └──────┘ └─────────┘  │
│  ┌─────────┐ ┌─────────┐ ┌────────────────┐              │
│  │ 动画    │ │ UI运行时│ │ 场景树/资源管理 │              │
│  │ 系统    │ │         │ │                │              │
│  └─────────┘ └─────────┘ └────────────────┘              │
├──────────────────────────────────────────────────────────-─┤
│                    平台抽象层                               │
│             Vulkan / Metal / OpenGL                        │
└──────────────────────────────────────────────────────────-─┘
```

---

## 游戏项目文件结构

### 标准目录结构

```
my_game/
├── project.json          ← 项目配置（入口脚本、入口场景、引擎版本等）
├── src/                  ← TypeScript 脚本（LLM 自由组织目录结构）
│   └── main.ts           ← 默认入口（可在 project.json 中覆盖）
├── scenes/               ← YAML 场景描述文件
│   └── main_scene.yaml
├── assets/               ← 资源文件
│   ├── models/
│   ├── textures/
│   ├── audio/
│   └── fonts/
├── config/               ← 游戏配置数据（关卡、平衡性参数等）
├── tsconfig.json
└── .eslintrc
```

### 设计决策

- `project.json` 定义项目元信息和入口脚本路径，引擎据此启动
- `src/` 下的目录结构由 LLM 自由组织，引擎不强制约定，但提供推荐结构模板
- 场景使用 YAML 格式描述（LLM 写 YAML 准确率高、可读性好），构建时通过 YAML-to-tscn 工具转换为 Godot 原生 `.tscn` 格式

### 构建管线

```
scenes/*.yaml  → YAML-to-tscn 工具 → .tscn → Godot ResourceLoader 加载
src/*.ts       → tsc + ESLint      → .js   → QuickJS 执行
assets/*       → 直接由 Godot ResourceLoader 加载
```

---

## 多模态反馈

### 第一版：截屏

- 低频自动截屏：每次编译运行后自动截一张，运行过程中按固定间隔（5-10 秒）截一张
- LLM 主动请求：通过 API 指定截屏参数

```ts
// 自动截屏随状态反馈一起返回
// LLM 也可主动请求截屏
Engine.screenshot(): ScreenCapture
Engine.screenshot({
    camera: "main_camera",      // 从哪个相机视角截
    resolution: [1280, 720],    // 分辨率
    ui_visible: true            // 是否包含 UI 层
}): ScreenCapture
```

### 后续扩展（暂不实现）

- 短视频录制：检查动画、粒子、运动轨迹等静态截图看不出的问题
- 音频波形/频谱快照：粗略判断音效时机

---

## 热重载策略

### 第一版：快速重启

代码变更后重新加载整个场景，不保留运行时状态。简单可靠，符合 LLM 增量迭代的工作方式。

### 后续扩展：TS 热重载

JS/TS 天然支持运行时模块替换，QuickJS 可以重新 eval 模块来替换旧的。当 LLM 需要调试长时间运行后才出现的问题时，再实现真热重载。

---

## 实施决策

| 决策项 | 选择 |
|--------|------|
| 优先目标平台 | macOS |
| Godot 基线版本 | 4.4 稳定版 |
| 上游同步策略 | 直接分叉，不跟进上游更新 |
| 协调层宿主 | 嵌入引擎进程内（非独立进程） |

## 测试策略

### Binding 生成器测试

挑选一组代表性 Godot 类（Node3D、MeshInstance3D、Camera3D、RigidBody3D 等），验证：
- 生成的 C++ binding 能编译通过
- 生成的 `.d.ts` 能通过 tsc 检查
- 两边的函数签名一致

### 运行时集成测试

编写 TS 测试脚本，在引擎 headless 模式下执行：

```ts
// test_node3d.ts
const node = scene.createNode3D("test");
node.setTransform({ position: new Vec3(1, 2, 3) });
assert(node.getTransform().position.x === 1);
```

- 利用 Godot 自带的 headless 模式，CI 中自动运行
- 优先覆盖 engine-core.d.ts 中的核心 API，不需要全量覆盖

---

## 待细化事项

- [ ] 代码生成器的具体实现（ClassDB 遍历、模板引擎选择）
- [ ] engine-core.d.ts 的核心 API 清单定义
- [ ] QuickJS 嵌入的具体集成点（Godot 主循环中的调用时机）
- [ ] 热重载机制（后续版本，基于 QuickJS 模块替换）
- [ ] 资源管线：
  - 复用 Godot ResourceLoader 体系（异步加载、缓存、引用计数）
  - 内置基础资产包：覆盖常见游戏类型的基础素材（角色、地形、建筑、UI 元素、音效等），随引擎分发
  - 资产清单 + 查询 API：构建时生成 asset_manifest.json，TS 侧提供 assets.list() / assets.info() 查询接口，让 LLM 知道有什么可用
  - 外部资产包接口：标准化资产包格式，支持导入第三方资产包，为 UGC 资产平台留接口
- [ ] 构建系统改造（SCons 配置裁剪、binding 生成集成到构建流程）
