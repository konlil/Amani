# 项目初始化与构建说明

本文档基于当前仓库的实际结构整理，目标是把这两个问题说清楚：

1. 如何初始化项目依赖
2. 如何构建项目输出

当前仓库已经明确分成两条构建链路：

- 引擎产物：`engine/godot` 中裁剪后的 Godot 引擎二进制
- 桌面端产物：`app` 中的 React + Vite + Tauri 桌面应用

## 1. 初始化项目依赖

### 1.1 系统前置依赖

当前可确认、且构建流程真实会使用到的依赖如下：

- macOS
- Xcode Command Line Tools
- Git
- Node.js 与 npm
- Rust toolchain（`cargo`）
- Python 3
- SCons
- MoltenVK

推荐先手动安装系统级工具：

```bash
xcode-select --install
brew install node python scons molten-vk
curl https://sh.rustup.rs -sSf | sh
```

说明：

- `SCons` 和 `MoltenVK` 用于构建 `engine/godot`
- `Node.js` 和 `npm` 用于安装 `app` 的前端/Tauri 依赖
- `Rust` 用于构建 `app/src-tauri`
- `Python 3` 是 Godot/SCons 构建链的基础依赖

### 1.2 初始化仓库依赖

仓库包含一个 Git submodule：

- `engine/godot` -> `godotengine/godot`，当前锁定在 `4.4.1-stable`

初始化命令：

```bash
git submodule update --init --recursive
cd app && npm ci
```

如果你还需要安装测试目录的 TypeScript 依赖，可以额外执行：

```bash
cd tests/e2e_test && npm ci
```

### 1.3 一键初始化脚本

仓库内已补充脚本：

```bash
./build/bootstrap_macos.sh
```

如果希望连 `tests/e2e_test` 的依赖也一起装上：

```bash
./build/bootstrap_macos.sh --with-tests
```

这个脚本会做四件事：

1. 校验 macOS 构建所需命令是否存在
2. 初始化 `engine/godot` submodule
3. 在 `app/` 下执行 `npm ci`
4. 按需在 `tests/e2e_test/` 下执行 `npm ci`

## 2. 构建项目输出

### 2.1 构建引擎产物

现有引擎构建脚本是：

```bash
./build/macos.sh
```

它本质上会在 `engine/godot` 目录执行 `scons`，并带上这几个关键参数：

- `platform=macos`
- `target=template_release`
- `tools=no`
- `custom_modules=<repo>/modules`
- `vulkan_sdk_path=/opt/homebrew/opt/molten-vk`

默认输出位置：

```bash
engine/godot/bin/
```

在 Apple Silicon 上，通常会看到类似下面的产物名：

```bash
engine/godot/bin/godot.macos.template_release.arm64
```

这个路径也正是桌面端设置页里需要填写的 `engine_path`。

### 2.2 构建桌面应用产物

桌面应用位于 `app/`，其中：

- 前端构建命令：`npm run build`
- Tauri 打包命令：`npm run tauri build`

直接构建桌面应用：

```bash
cd app
npm run tauri build
```

构建时会自动执行：

1. `npm run build`
2. Rust / Tauri 编译
3. 生成桌面应用 bundle

默认输出目录：

```bash
app/src-tauri/target/release/bundle/
```

### 2.3 一键构建脚本

仓库内已补充脚本：

```bash
./build/build_all_macos.sh
```

默认行为：

1. 先调用 `./build/macos.sh` 构建 Godot 引擎
2. 再进入 `app/` 执行 `npm run tauri build`
3. 最后打印引擎产物路径和桌面应用 bundle 目录

可选参数：

```bash
./build/build_all_macos.sh --engine-only
./build/build_all_macos.sh --app-only
./build/build_all_macos.sh --target template_debug
```

## 3. 推荐执行顺序

建议按下面顺序操作：

```bash
./build/bootstrap_macos.sh
./build/build_all_macos.sh
```

完成后重点查看两个输出位置：

- 引擎二进制：`engine/godot/bin/`
- 桌面应用 bundle：`app/src-tauri/target/release/bundle/`

## 4. 当前项目的几个关键注意点

### 4.1 这是一个以 macOS 为主的构建链路

目前仓库里唯一现成的构建脚本是 `build/macos.sh`，而 Tauri 侧还有 macOS 专用的 overlay 预览代码，因此当前最稳妥、也是文档覆盖最完整的路径就是 macOS。

### 4.2 当前预览区采用 overlay 模式，而不是真嵌入

当前实现里，Godot 仍然是独立窗口，但会以无边框模式运行，并通过 Tauri 计算预览区的屏幕坐标来贴合右侧预览面板。

这意味着：

- 视觉效果上接近“内嵌预览”
- 实现上不是 `NSView` 父子窗口嵌入
- Tauri 与 Godot 通过本地 IPC 长连接同步窗口位置、显示隐藏和运行命令

如果你在文档或代码里看到更早期的 “WebSocket” 或 “NSView 嵌入” 表述，应以当前 overlay 模式实现为准。

### 4.3 游戏项目的 TypeScript 编译依赖 `app/node_modules`

`app/src-tauri/src/project.rs` 中的 `compile_project()` 会优先调用：

```bash
app/node_modules/.bin/tsc
```

这意味着即使只是想在应用里编译游戏脚本，也必须先执行 `cd app && npm ci`。

### 4.4 引擎产物不会自动打进桌面应用里

当前代码里，Tauri 应用运行时通过用户填写的 `engine_path` 去启动 Godot 引擎二进制。因此当前流程是：

1. 先单独构建引擎
2. 再构建桌面应用
3. 运行应用后，在设置中填写引擎二进制路径
