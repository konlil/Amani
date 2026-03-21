# LLM3dEngine 架构设计文档

## 项目定位

一个专为大语言模型（LLM）设计的 3D 游戏引擎。LLM 作为开发者，通过编写代码来构建完整游戏。

## 核心设计原则

- 底层扎实：引擎核心基于各平台原生图形 API（Vulkan/Metal/DirectX），由人类工程师编写，经过编译器验证和测试覆盖
- API 正式化：暴露给 LLM 的接口是强类型、可编译验证的，不使用语义化魔法 API
- 容错设计：LLM 会犯错，引擎通过编译检查和 linter 在执行前拦截错误
- 反馈驱动：结构化的错误信息和状态报告，让 LLM 能自主修复问题

---

## 整体架构

```
┌──────────────────────────────────────────────────────┐
│                     LLM 端                            │
│  接收需求 → 查阅 .d.ts → 生成/修改 .ts 文件            │
│       ↑                                               │
│       │ 结构化反馈（编译错误/运行状态/截屏/人类反馈）      │
├───────┼──────────────────────────────────────────────-─┤
│       │            引擎协调层                           │
│       │                                               │
│  tsc 编译 → ESLint 检查 → QuickJS/V8 执行              │
│       │                      ↓                        │
│       └────── 状态收集 ← 引擎运行时                     │
│              (JSON + 截屏)                             │
├──────────────────────────────────────────────────────-─┤
│                引擎核心 (C++)                           │
│    渲染 / 物理 / 音频 / 场景管理 / 资源管理              │
├──────────────────────────────────────────────────────-─┤
│                 平台抽象层                              │
│          Vulkan / Metal / DirectX / OpenGL             │
└──────────────────────────────────────────────────────-─┘
```

### 各层职责

**LLM 端**
- 接收游戏需求，查阅引擎 API 类型声明（`.d.ts`），生成和修改 TypeScript 脚本
- 接收引擎反馈（编译错误、运行时状态、截屏、人类反馈），迭代修改代码

**引擎协调层**
- 编译管线：tsc 类型检查 → ESLint 自定义规则检查 → 编译为 JS
- 执行管线：将 JS 交给 QuickJS/V8 运行
- 状态收集：从引擎运行时提取结构化状态和截屏，反馈给 LLM

**引擎核心（C++）**
- 渲染、物理、音频、场景管理、资源管理等传统引擎功能
- 通过 binding 层向 TypeScript 脚本暴露 API

**平台抽象层**
- 封装各平台原生图形 API，提供统一接口

---

## 脚本层设计

### 语言选择：TypeScript

选择理由：
- LLM 生成 TypeScript 代码的准确率在所有语言中最高
- 强类型系统在编译期拦截错误，错误信息结构化且 LLM 易于理解
- ESLint 生态成熟，自定义规则实施成本低
- tsc 增量编译速度快，适合高频迭代

### 运行时策略

- 开发阶段：QuickJS（轻量、快启动、嵌入简单，约几百 KB）
- 发布阶段：V8（JIT 编译，性能比 QuickJS 快 10-100 倍）
- 脚本层 API binding 做抽象，底层运行时可切换，上层 TS 代码不受影响

### 语言特性约束

采用"现有语言 + linter 硬检查"策略，而非自造 DSL：

通过 `tsconfig.json` 严格模式 + ESLint 自定义规则，禁止以下特性：
- `any` 类型（`noImplicitAny` + `@typescript-eslint/no-explicit-any`）
- `eval` / `Function` 构造器（`no-eval` / `no-new-func`）
- 动态 import
- 其他不安全特性按需加入黑名单

禁用这些特性对 LLM 开发游戏的影响极小：
- 无动态类型：LLM 不怕写类型声明，强类型反而是安全网
- 无 eval：LLM 的工作流本身就是"改源码 → 重编译"，不需要运行时动态执行
- 无反射：LLM 本身就是样板代码生成器，手写序列化等代码毫无压力

数据驱动场景通过引擎层提供类型安全的 tagged union 解决：
```ts
type Value = { type: "int", value: number }
             | { type: "float", value: number }
             | { type: "string", value: string }
             | { type: "list", value: Value[] }
             | { type: "map", value: Record<string, Value> }
```

### API 暴露方式

通过 `.d.ts` 类型声明文件，既是 LLM 的 API 文档，又是 tsc 的类型检查依据：

```ts
// engine.d.ts
declare namespace Engine {
    interface Scene {
        createEntity(name: string): Entity;
        findEntity(name: string): Entity | null;
        getEntities(): Entity[];
    }

    interface Entity {
        setMesh(path: string): void;
        setTransform(t: Transform): void;
        addComponent<T extends Component>(type: ComponentType<T>): T;
    }

    interface Transform {
        position: Vec3;
        rotation: Quat;
        scale: Vec3;
    }
}
```

API 设计原则：
- 命名极度一致和可预测（LLM 靠模式匹配写代码）
- 每个 API 都有机器可读的 schema

---

## LLM 工作流设计

### 完整工作流

```
1. LLM 接到需求（如"做一个第一人称射击游戏原型"）
2. LLM 查阅引擎 .d.ts，了解可用能力
3. LLM 生成项目结构和 TS 脚本
4. 引擎编译（tsc + ESLint）
5. 通过 → 运行 → 引擎返回运行状态
   失败 → 结构化错误信息反馈给 LLM → 回到第 3 步
6. LLM "观察" 运行结果（状态 JSON + 截屏），决定下一步修改
7. 到达检查点时请求人类反馈
8. 循环迭代直到完成
```

### 增量式开发

LLM 不应一次生成整个游戏，而是增量迭代：

```
阶段 1：生成空场景 + 基础相机 → 编译运行 → 确认能跑
阶段 2：加地形和光照 → 编译运行 → 检查渲染
阶段 3：加玩家控制器 → 编译运行 → 检查移动
阶段 4：加游戏逻辑 → 编译运行 → 检查交互
...
```

引擎需支持热重载或快速重启，LLM 改一个文件不需要重新加载整个场景。

### 结构化反馈

#### 编译期错误

```json
{
    "stage": "compile",
    "file": "player_controller.ts",
    "line": 42,
    "message": "Property 'velocty' does not exist on type 'RigidBody'. Did you mean 'velocity'?",
    "suggestion": "velocity"
}
```

#### 运行时错误

```json
{
    "stage": "runtime",
    "file": "enemy_ai.ts",
    "line": 15,
    "message": "Cannot call 'getComponent' on destroyed entity 'enemy_03'",
    "context": { "tick": 1204, "scene_state": "..." }
}
```

#### 运行状态

```json
{
    "entities": [
        { "name": "player", "position": [0, 1.8, 0], "components": ["Camera", "CharacterController"] },
        { "name": "enemy_01", "position": [10, 0, 5], "components": ["Mesh", "AIAgent", "Health(80)"] }
    ],
    "fps": 60,
    "physics": { "activeCollisions": 2 },
    "errors": [],
    "warnings": ["entity 'light_02' has no shadow caster"]
}
```

#### 截屏

引擎定时截屏，转成图片喂给多模态 LLM，用于粗粒度视觉检查（穿模、物体缺失等）。

---

## 人机协作：检查点机制

### 设计理念

LLM 能自主处理的事情不打扰人类，搞不定的精准提问。

### 两类反馈来源

**LLM 自主处理：**
- 编译错误、运行时错误
- 场景结构合理性（实体关系、组件配置）
- 性能指标（帧率、内存）
- 截屏粗粒度检查（穿模、物体明显缺失）

**需要人类介入：**
- 美术感受（光照氛围、颜色搭配）
- 音效体验（节奏、情绪匹配）
- 操控手感（灵敏度、镜头舒适度）
- 整体"好不好玩"

### 检查点触发策略

- 阶段性触发：每完成一个大阶段自动暂停（场景搭建完、角色能动了、核心玩法跑通了）
- LLM 主动请求：LLM 判断"这个我没把握"时主动要求人类看一眼
- 人类随时打断：人类在预览窗口里随时可以暂停并给反馈

### 检查点 API

```ts
Engine.checkpoint({
    stage: "scene_setup",
    description: "基础场景搭建完成",
    preview: true,           // 自动打开预览窗口
    questions: [             // LLM 想问人类的具体问题
        "光照氛围是否符合预期？",
        "地形比例是否合适？"
    ]
}): HumanFeedback           // 阻塞，等人类回复
```

---

## 待讨论事项

- [x] 引擎核心架构细节 → 详见 [ENGINE_FOUNDATION.md](ENGINE_FOUNDATION.md)
- [x] TS API binding 层具体实现方案 → 详见 [ENGINE_FOUNDATION.md](ENGINE_FOUNDATION.md)
- [x] 资源管线设计 → 详见 [ENGINE_FOUNDATION.md](ENGINE_FOUNDATION.md) 待细化事项
- [x] 项目文件结构规范 → 详见 [ENGINE_FOUNDATION.md](ENGINE_FOUNDATION.md)
- [x] 多模态反馈的具体实现 → 详见 [ENGINE_FOUNDATION.md](ENGINE_FOUNDATION.md)
- [x] 热重载机制 → 详见 [ENGINE_FOUNDATION.md](ENGINE_FOUNDATION.md)（第一版快速重启，后续扩展热重载）
