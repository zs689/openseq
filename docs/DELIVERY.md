# 交付说明 / Delivery notes

**任务**：用 AI 辅助编程，以 **Rust + TypeScript + Tauri** 技术栈重构 `github.com/logseq/logseq`，实现功能一致。

**结论**：Logseq 全量功能不可能一次交付（核心是 ClojureScript，约 25,861 次提交，含 DataScript、
Electron、移动端、插件系统、加密同步）。本次交付的是**核心的完整重写**，并且
**Rust 核心 26 个测试全部通过、Tauri 桌面程序已能编译并实际启动，NSIS 安装包端到端验证过**。不是"Logseq 1:1"。

---

## 一、验证证据（逐条可复现）

| # | 检查 | 命令 | 结果 |
| --- | --- | --- | --- |
| 1 | Rust 编译（含测试目标） | `cargo check -p logseq-core --all-targets` | **通过，0 warning** |
| 2 | Rust 单元测试 | `cargo test -p logseq-core` | **16 passed / 0 failed** |
| 3 | Rust 集成测试 | 同上 | **10 passed / 0 failed** |
| 4 | Rust 静态检查 | `cargo clippy -p logseq-core --all-targets -- -D warnings` | **0 lint** |
| 5 | TS 类型检查 | `node node_modules/typescript/bin/tsc --noEmit` | **0 error** |
| 6 | 前端生产构建 | `node node_modules/vite/bin/vite.js build` | **成功**，62 modules |
| 7 | 界面实际渲染 | Python 静态服务 + Edge 无头截图 | **通过**，`docs/screenshot-app.png`、`docs/screenshot-graph.png` |
| 8 | **Tauri 外壳编译** | `cargo build -p openseq`（debug）与 `tauri build`（release） | **均成功**，0 warning。debug 227 MB；release 独立版 **4.4 MB** |
| 9 | **桌面程序实际启动** | 运行 `openseq.exe`（release 独立版），并经用户手动启动 | **通过**，窗口正常渲染，见 `docs/screenshot-desktop.png` |
| 10 | **安装包打包** | `tauri build --bundles nsis,msi` | **成功，两个包**（见下） |
| 11 | **NSIS 安装包端到端** | 静默安装 → 启动 → 卸载 | **通过**，见下 |
| 12 | **MSI 安装包结构** | 读 MSI 数据库核对产品信息与载荷 | **通过**，但**未实际安装**（需管理员） |

安装包（一条命令同时产出，可复现）：

```
E:\git\logseq-tauri\target\release\bundle\nsis\OpenSeq_0.1.0_x64-setup.exe   1.55 MiB  (MZ / PE)
E:\git\logseq-tauri\target\release\bundle\msi\OpenSeq_0.1.0_x64_en-US.msi    2.22 MiB  (OLE2/CFBF)
```

**下载链接**：https://openseq-download.app.workbuddy.host/

### 第 11 / 12 条的具体证据（安装包真的验证过）

**NSIS（`OpenSeq_0.1.0_x64-setup.exe`）—— 完整走了一遍安装/卸载**：

1. 静默安装 `setup.exe /S`，退出码 **0**；
2. 产物落在 `%LOCALAPPDATA%\OpenSeq\`：`openseq.exe`（4,635,648 B）、
   `WebView2Loader.dll`、`uninstall.exe`。（`/D=` 指定目录**无效** ——
   Tauri 的 `.onInit` 自己设 `$INSTDIR`，安装模式是 per-user。）
3. **载荷字节级核对**：安装出来的 `openseq.exe` 与 `target/release/openseq.exe`
   **只差 3 个字节**，位置 4361613–4361615，上下文是
   `...__TAURI_BUNDLE_TYPE_VAR_UNK` → `...__TAURI_BUNDLE_TYPE_VAR_NSS`。
   也就是说：**除了 Tauri 写入的 bundle 类型标记（UNK→NSS），两者完全同一份二进制。**
4. 启动安装出来的 `openseq.exe`：进程正常起来、无任何错误输出、
   `%LOCALAPPDATA%\io.openseq.app\EBWebView` 正常创建；
5. 跑 `uninstall.exe /S`：`%LOCALAPPDATA%\OpenSeq\` **被删干净**。

> 注：第 4 步只能证明"进程起来了"，**不能**证明窗口渲染正常 ——
> 我自己的沙箱会让 WebView2 渲染进程崩溃（见下），所以画面得由用户肉眼确认。

**MSI（`OpenSeq_0.1.0_x64_en-US.msi`）—— 只读了数据库，没有安装**：

| 属性 | 值 |
| --- | --- |
| ProductName / Version | `OpenSeq` / `0.1.0` |
| Manufacturer | `openseq` |
| **ALLUSERS** | **1 → 每机器安装，需要管理员权限** |
| 载荷 | `openseq.exe`（4,635,648 B）、`WebView2Loader.dll`（160,880 B） |
| 安装位置 | `Program Files\OpenSeq`，含开始菜单 + 桌面快捷方式 + 卸载项 |

两个包里 `openseq.exe` 的**字节数与构建产物完全一致**（4,635,648 B）。

**没有做的**：MSI **没有真的装过**（它要管理员权限，我不主动提权装东西）。

### 第 9 条的具体证据（不是"应该能跑"）

启动 `openseq.exe` 之后（debug 版与 **release 版 4.4 MB** 都验证过）：

1. 进程在 **Session 5 (Console)** 稳定存活（观察 14 秒以上，无任何错误输出）；
2. `%LOCALAPPDATA%\io.openseq.app\EBWebView\` 被**创建并写入** ——
   这是 WebView2 **只有真正初始化后才会生成**的用户数据目录；
3. 其下 `Default/` 里有完整的 Chromium profile（`Cache`、`Code Cache`、`DawnGraphiteCache`、
   `Favicons`、`Extension Rules` …）。

也就是说：**窗口建起来了，WebView2 环境初始化了，前端页面被加载了。**

**当前正在运行的就是 `tauri build` 产出的独立版**（`target/release/openseq.exe`，4.4 MB）。
它在启动时**没有**向 `localhost:1420` 发任何请求 —— 我用「在 1420 放一个记录请求的服务器」
的方式实测过，所以能确认它用的是内嵌前端，而不是开发服务器。
（`target/debug/openseq.exe` 是 dev 模式，`cargo:dev=true`，必须配 dev 服务器才能出画面。）

**桌面窗口的真实画面**：`docs/screenshot-desktop.png`（由用户在本机手动启动
`target/release/openseq.exe` 后截取）。可以看到：原生 Windows 标题栏（最小化/最大化/关闭）、
左侧 JOURNALS 与 PAGES 列表（含各页块数）、右侧图谱视图把 4 个页面 + 2 篇日记
及它们之间的连线渲染出来、底部状态栏「本地优先 · 无账号 · 不上传」。

**一个必须说清楚的插曲**：我最初**在自动化沙箱里**启动这个 exe 时，看到的是一个**纯白窗口**，
而且 WebView2 渲染进程会崩溃。我据此一度怀疑是 OpenSeq 自己的问题，做了大量排查
（dev 模式、CSP、前端资源有没有内嵌、网络、前端代码……全部排除）。最后定位到**根因不在 OpenSeq**：
沙箱会往子进程注入一个钩子 DLL，它把 WebView2 的渲染进程搞崩了。证据有三条：

1. 一个**不含任何应用代码**的空页面，在沙箱里也照样会崩；
2. 崩溃转储（`%LOCALAPPDATA%\io.openseq.app\EBWebView\Crashpad\reports\`，MDMP 格式）
   的模块列表里都能看到那个注入 DLL；
3. **用户在自己的正常环境里启动同一个 exe，界面完全正常**（即上面那张截图）。

所以：**沙箱环境不适合用来跑 GUI 程序**，这是环境限制，不是产品缺陷。

### 测试抓到了一个真 bug（这不是修辞）

第一次 `cargo test` 时 `parser::tests::parses_marker_and_priority` **失败**：

```
assertion `left == right` failed
  left: ""
 right: "ship it"
```

`- TODO [#A] ship it` 解析出来的块正文是**空的**。原因：正则
`^\[#([A-Za-z0-9]+)\][ \t]*(.*)$` 里的 `(.*)$` 已经吞到行尾，而我按"整个匹配之后"去切片，
把正文也切掉了。**`cargo check` 完全发现不了**（无类型错误、无借用错误）。修好后测试全绿。

---

## 二、为了"能在本机构建"，解决了 8 个具体问题

本机没有 MSVC，也没有任何 MinGW，所以 `x86_64-pc-windows-gnu` 是唯一出路。
一路踩下来，每个问题都定位到了根因（不是"重装试试"）：

| # | 现象 | 根因 | 处理 |
| --- | --- | --- | --- |
| 1 | 没有 Rust 工具链 | —— | 用清华镜像装到工作区内 `E:/git/.rust`（官方源约 2MB/min，太慢） |
| 2 | `cargo/bin/*.exe` 全是 0 字节，报 `不是有效的 Win32 程序 (os error 193)` | 首次安装被中断，rustup 的 shim 没写成功 | 删空壳，把 `rustup.exe` 复制成 `cargo.exe`/`rustc.exe`/… 13 个名字（rustup 本来就是这么做的）。**注意：不能直接 cp 工具链里的真二进制** —— rustc 按相对路径找 sysroot，会报 `E0463 can't find crate for std` |
| 3 | `cargo test` 报 `error calling dlltool 'dlltool.exe': program not found` | 自举工具链**有** dlltool/ld 但**没有汇编器 `as.exe`**；`windows-sys ≥ 0.59` / `getrandom` 用 `raw-dylib`，要求编译期现造导入库，dlltool 会去调 `as` | 从清华 MSYS2 镜像取 `mingw-w64-x86_64-binutils`（6.6MB，8.5MB/s）放进 `E:/git/.mingw` |
| 4 | 仍被 `raw-dylib` 拖住 | `walkdir`（主依赖）、`uuid` + `tempfile` 把 `windows-sys 0.61` 和 `getrandom` 拉了进来 | **精简依赖**：`walkdir` → 20 行 std 递归；`uuid` → 新增 `src/id.rs`（v4 形状，2 万次无碰撞，有测试）；`tempfile` → 测试内手写 `TempDir` |
| 5 | `schemars 0.8.22` 编译失败 `E0107: Map<K,V>` 少泛型参数 | `indexmap 1.9.3` 的 `[features]` **没有 `default`**，`std` 必须显式开启；schemars 声明了 indexmap 却没开 `std`，于是 indexmap 的构建脚本退化成"探测目标是否支持 std"，探测返回 false | 在 `src-tauri` 的 **build-dependencies** 里显式启用 `indexmap/std`（indexmap 构建脚本原文就写着"若显式请求 std 就不必探测"） |
| 6 | `tauri-build` 构建脚本静默退出 101，看不到原因 | cargo 吞掉了它的 stderr | 临时用 `catch_unwind` 把 panic 抓出来写文件，才看到真因（第 7 条） |
| 7 | `windres: preprocessing failed` | `windres` 用 **`gcc -E`** 预处理 `.rc`，而 PATH 上只有 binutils 没有 gcc；Rust 自举的那个 gcc 是"只能当链接器"的阉割版（报 `cannot execute 'cc1'`） | 从 MSYS2 取 `gcc` + `gmp/mpfr/mpc/isl/libwinpthread/libiconv/gettext-runtime/zlib-ng` + `libgcc_s_seh-1.dll` |
| 8 | 链接失败 `ld: error: export ordinal too large: 97673` | `[lib] crate-type` 里的 `cdylib` 是给 Tauri 移动端用的；桌面上它让 GNU ld 为整个依赖图生成导出表，超出序号上限 | 改为 `crate-type = ["lib"]`（要打移动端时再加回 `staticlib`/`cdylib`） |

**最终 `cargo build -p openseq` 成功，无警告。**

---

## 三、本机环境（下次直接用，别重复踩）

工具链全部装在工作区内，**没有污染系统**：

| 路径 | 内容 |
| --- | --- |
| `E:/git/.rust` | Rust 工具链（`rustc 1.99.0`，target `x86_64-pc-windows-gnu`，清华镜像） |
| `E:/git/.mingw/extract/mingw64` | MinGW binutils + gcc（`as` `ld` `dlltool` `windres` `gcc`） |

**一键环境**：`source scripts/rust-env.sh`（或 `scripts\rust-env.cmd`）。
它会设好 `CARGO_HOME` / `RUSTUP_HOME`、把 MinGW 与 cargo 加进 PATH，
并启用 `RUSTFLAGS="-C link-self-contained=yes"`。

**前端**：本机 Node 无法派生子进程（`spawnSync` 直接 `EBUSY`），所以必须
`npm install --ignore-scripts`；跑工具用 `node node_modules/typescript/bin/tsc` /
`node node_modules/vite/bin/vite.js`，**不要走 `npm run`**。

**代理**：`HTTP_PROXY=http://127.0.0.1:58442`。curl 访问 127.0.0.1 要 `--noproxy '*'`；
Edge/Chrome 类工具加 `--no-proxy-server`。

---

## 四、功能对照

| Logseq 能力 | 本实现 |
| --- | --- |
| 本地 Markdown 图谱（`pages/` `journals/` `assets/`） | ✅ |
| 文件名 ↔ 页面名 编解码（`a/b` ↔ `a%2Fb.md`；日记 `2026_10_11.md`） | ✅ |
| 块级大纲（嵌套 / Tab / Shift+Tab / 合并 / 折叠） | ✅ |
| TODO·DOING·DONE 等 7 种标记 + `[#A]` 优先级 | ✅ |
| 块属性与页面属性 `key:: value` | ✅ |
| `[[页面链接]]` / `#标签` | ✅ |
| `((块引用))`（`id::` 持久化，可跨会话） | ✅ |
| 反向引用 Linked References | ✅ |
| 日记 / 每日笔记 | ✅ |
| 全文搜索（中文用单字 + 相邻二元组，无需分词词典） | ✅ |
| 图谱视图（手写力导向 + Canvas，可拖拽） | ✅ |
| 文件外部改动感知（`notify` → `graph-changed`） | ✅ |
| 原子写盘（临时文件 + rename） | ✅ |
| 插件系统 / 白板 / 卡片复习 | ❌ 未实现 |
| 查询 DSL（Datalog `{{query}}`） | ❌ 未实现 |
| 加密同步 / 多人协作 / 移动端 | ❌ 未实现 |
| PDF 标注 / Zotero / 插件市场 | ❌ 未实现 |
| Org-mode（`*.org`） | ❌ 仅 Markdown |

---

## 五、怎么跑

```bash
cd E:/git/logseq-tauri
source scripts/rust-env.sh

# 1) Rust 核心：26 个测试
cd crates/core && cargo test

# 2) 前端（本机必须 --ignore-scripts）
npm install --ignore-scripts
npm run dev                       # 浏览器演示后端（不需要 Rust）

# 3) 桌面程序
#
#    注意：dev 和「独立可运行」是两种构建，别搞混。
#    tauri/build.rs 里是 `dev = !custom_protocol`，只有给 tauri crate 打开
#    `custom-protocol` 特性，前端资源才会被内嵌、程序才能单文件运行。
#
# (a) 开发模式 —— 需要同时有 vite 开发服务器（默认 :1420），否则窗口是空白
node node_modules/vite/bin/vite.js &     # 或 npm run dev
cargo build -p openseq                   # → target/debug/openseq.exe

# (b) 独立可运行 / 打安装包 —— 前端内嵌，CLI 内部就是传 --features tauri/custom-protocol
node node_modules/@tauri-apps/cli/tauri.js build --bundles nsis,msi \
  --config '{"build":{"beforeBuildCommand":null}}'
# → target/release/bundle/nsis/OpenSeq_0.1.0_x64-setup.exe  (1.55 MiB)
# → target/release/bundle/msi/OpenSeq_0.1.0_x64_en-US.msi   (2.22 MiB)
#   同时产出 target/release/openseq.exe（独立可运行）

# 不用 CLI 时的等价写法：
cargo build -p openseq --release --features tauri/custom-protocol
```

> ⚠️ **裸跑 `cargo build -p openseq --release` 得到的不是独立程序** ——
> 它是 dev 模式，启动时会去连 `http://localhost:1420`，那里没有开发服务器就是白屏。
> 这一点我实测确认过：在 1420 上放一个记录请求的服务器，`tauri build` 产出的 exe
> 全程**没有发起任何请求**（说明用的是内嵌资源），而 debug 构建的 `cargo:dev=true`。

打包这一步有两个本机特有的坑，都在命令里解决了：

- **必须清掉 `beforeBuildCommand`**（配置里是 `npm run build`）。本机 Node 不能派生子进程，
  `npm run` 一定失败。前端已经用 `vite build` 单独构建好了（`dist/` 存在），所以直接跳过。
- **必须设 `TAURI_BUNDLER_TOOLS_GITHUB_MIRROR`**。打包器要从 GitHub Releases 下载
  NSIS 与 `nsis_tauri_utils.dll`，而本机**直连 github.com 完全不通**（`curl` 返回 `http=000`）。
  用镜像即可：`export TAURI_BUNDLER_TOOLS_GITHUB_MIRROR="https://ghfast.top"`。
  注意拼接形式是**把完整 URL 接在镜像后面**
  （即 `https://ghfast.top/https://github.com/...`）；
  只传路径的形式会被 403。

> 用 `npm run tauri build` 的写法在正常机器上是等价的；本机要按上面两条绕过。

---

## 六、遗留文件

| 路径 | 大小 | 状态 |
| --- | --- | --- |
| `E:/git/.rust` | ~2.6 GB | **保留** —— 能跑测试与构建的 Rust 工具链 |
| `E:/git/.mingw` | ~120 MB | **保留** —— binutils + gcc；缺了它 `raw-dylib` 和 `windres` 都会失败 |
| `E:/git/logseq-tauri/target` | 数 GB | 构建产物，已在 `.gitignore`；可随时删（删了要重编译） |
| `E:/git/logseq-tauri/.node_modules_broken` | 53 MB | 第一次 npm 失败残留，**可删**（批量删除被安全策略拦截，需你确认） |

已清理：`E:/git/_dlltest`、`E:/git/.rust/mingw.zip`，以及诊断用的
`_bsenv.txt` / `_bsresult.txt` / `_win.txt` / `_win2.txt`。
