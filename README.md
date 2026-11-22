<div align="center">

<img src="assets/logo.svg" alt="Bento Logo" width="112" />

# Bento

**专为 NSH 桌面工具家族打造的原生 UI 组件库、统一图标与桌面壳基础设施**

为 **Wonton**、**Sundae** 等原生桌面工具提供 100% 语义化的主题组件层、统一 24×24 矢量图标库、隐藏标题栏与系统托盘桌面壳、合成器驱动的拖拽排序控制器以及 GitHub Releases 更新检测能力。  
基于 **Rust + [NSH](https://github.com/longbridge/nsh)** 构建，纯原生渲染，无 JavaScript 运行时开销。

<p>
  <a href="https://github.com/Summe-rsnow/Bento/releases"><img src="https://img.shields.io/badge/Version-v1.0.0-10b981?style=flat-square" alt="Version v1.0.0" /></a>
  <img src="https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust" alt="Rust 2024" />
  <img src="https://img.shields.io/badge/Platform-Windows-0ea5e9?style=flat-square" alt="Platform Windows" />
  <img src="https://img.shields.io/badge/UI-NSH%20Native-6366f1?style=flat-square" alt="UI NSH Native" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-146c2e?style=flat-square" alt="License Apache-2.0" /></a>
</p>

<p>
  <a href="#-家族工具集">家族工具集</a> ·
  <a href="#-核心模块">核心模块</a> ·
  <a href="#-统一图标管理">统一图标管理</a> ·
  <a href="#-快速开始">快速开始</a> ·
  <a href="#-项目结构">项目结构</a> ·
  <a href="https://github.com/Summe-rsnow/Bento">GitHub</a>
</p>

</div>

---

## 🍱 家族工具集

| 工具 | 当前版本 | 仓库地址 | 定位与职责 |
|---|---|---|---|
| **Bento** | `v1.0.0` | [Summe-rsnow/Bento](https://github.com/Summe-rsnow/Bento) | 共享原生 UI 组件库、统一 24×24 图标库与桌面壳基础设施 |
| **Wonton** | `v1.2.1` | [Summe-rsnow/Wonton](https://github.com/Summe-rsnow/Wonton) | 专为 Antigravity 打造的进程级独立代理与多账号额度管理工具 |
| **Sundae** | `v1.0.0` | [Summe-rsnow/Sundae](https://github.com/Summe-rsnow/Sundae) | 专为 Windows Scoop 打造的轻快原生桌面软件包管理器 |

---

## ✨ 核心模块

- **🧩 `ui` — 语义化原生组件层**
  - 完全基于 `nsh_base` 与 `theme()` 构建，零硬编码色值，天然适配浅色 / 深色双主题与键盘 `focus_visible` 焦点环（`focus_ring`）。
  - 覆盖按钮与工具栏操作（`button`、`action`、`action_disabled`、`action_sm`、`action_with_icon`、`icon_action`、`icon_action_with_tip`、`icon_action_spinning_with_tip`、`segmented_icon`、`hint_tooltip`、`external_link`、`external_link_button`、`ButtonKind`、`IconKind`）、表单输入（`text_field`、`textarea`、`search_input`、`checkbox`、`toggle_switch`、`radio_option`）、导航控件（`sidebar_nav_item`、`filter_chip`）、进度条（`progress_bar`）以及表面与弹窗容器（`card`、`divider`、`toolbar_separator`、`section_header`、`empty_state`、`status_badge`、`notice_toast`、`modal_dialog`、`progress_modal_dialog`、`meta_row`、`confirm_dialog`）。
  - 内置 `ValueTone` 语义色调映射、`mono_font()` 等宽字体读取、统一圆角 `radius()` 以及 `MOTION_INSTANT_MS` / `MOTION_FAST_MS` / `MOTION_BASE_MS` / `MOTION_SLOW_MS` 动效时长常量。

- **🎯 `ui::icons` — 统一家族 24×24 矢量图标与品牌标**
  - 集中维护 44 个统一风格的 24×24 `currentColor` 矢量图标常量（涵盖通用操作、导航、代理网络、Scoop 仓库源、客户端启动等场景），并通过 `svg_icon` 动态绑定语义主题色。
  - 提供 `brand_mark_svg` 与 `bento_brand_mark`，保留应用 `assets/logo-mark.svg` 原生多色渐变而不覆盖内部色彩；同时提供 `png_data_uri` 将嵌入式 PNG 编码为系统托盘图标可用的 `ImageSrc`。

- **🪟 `shell` — 桌面壳、托盘与系统集成**
  - **隐藏标题栏框架**：`init_desktop_window`、`title_bar_shell`、`title_bar_brand`、`title_bar_brand_clickable` 与带未读更新圆点的 `about_icon_button`，自动预留各平台原生窗口按钮安全边距并支持窗口拖拽。
  - **外观与关闭行为**：`theme_mode_segmented` 配合 `ThemeMode`（`from_key` / `as_key` / `apply` / `label`）实现「跟随系统 / 浅色 / 深色」三段切换；`bind_close_handler`、`close_behavior_submenu`、`confirm_close_dialog` 配合 `CloseBehavior`（`Ask` / `Tray` / `Exit`）统一处理关闭最小化到托盘或直接退出。
  - **悬浮反馈与本地打开**：`NoticeCenter`（配合 `NoticeTone`，普通提示 3 秒自动消失、错误提示 6 秒）提供不占布局空间的底部悬浮通知；`open_directory` 与 `open_local_file` 跨平台调用系统文件管理器与默认应用打开本地目录或文件（绕过 NSH 0.6.1 `open_url` 对 Windows 盘符路径的限制）。

- **🧲 `drag_list` — 合成器驱动的拖拽排序控制器**
  - `DragList` 负责垂直卡片列表的拖拽排序状态管理、槽位高度与间距推断、滞回阈值防抖以及 `AnimatedValue` 位移插值，自动尊重系统 `reduce_motion()` 减弱动态效果设置。

- **🔄 `updater` — 应用元信息与 GitHub 更新检测**
  - 通过 `AppMeta` 描述应用名称、版本、标语、仓库 Slug、开源协议与品牌标构造器，配合 `check_latest_release`（支持代理）完成 GitHub Releases API 拉取与 SemVer 比较，并由 `about_dialog` 统一呈现「关于与检查更新」模态弹窗。

- **🎨 `theme.json`（`DEFAULT_THEME_JSON`）**
  - 家族共用的浅色 / 深色语义色 Token 配置，在基础表面、强调色与边框之外扩展了 `success`、`warning`、`info`、`sidebar`、`terminal` 与 `scrim` 语义 Token。

---

## 🎨 统一图标管理

家族应用无需各自拷贝或维护几十个通用 SVG 文件，只需在自己的 `src/ui/icons.rs` 中一行 re-export `bento::ui::icons::*`，并专心托管自身品牌标与托盘图标即可：

```rust
use std::sync::OnceLock;

pub use bento::ui::icons::*;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

pub const BRAND_MARK: &str = include_str!("../../assets/logo-mark.svg");
const TRAY_ICON_PNG: &[u8] = include_bytes!("../../assets/tray-icon.png");

/// 使用 Bento 辅助函数渲染保留多色渐变的完整品牌标
pub fn brand_mark(size: f32) -> impl IntoElement {
    bento::ui::icons::brand_mark_svg(BRAND_MARK, size)
}

/// 缓存编码 64×64 系统托盘图标 Data URI
pub fn tray_icon_src() -> ImageSrc {
    static DATA_URI: OnceLock<ImageSrc> = OnceLock::new();
    DATA_URI
        .get_or_init(|| bento::ui::png_data_uri(TRAY_ICON_PNG))
        .clone()
}
```

---

## 🚀 快速开始

### 在家族应用中引入 Bento

在应用的 `Cargo.toml` 中通过 Git Tag 引入稳定版本 `v1.0.0`：

```toml
[dependencies]
bento = { git = "https://github.com/Summe-rsnow/Bento.git", tag = "v1.0.0" }
```

在同级目录同步开发与联调 `Bento` 时，可通过 `[patch]` 临时指向本地工作区：

```toml
[patch."https://github.com/Summe-rsnow/Bento.git"]
bento = { path = "../Bento" }
```

### 运行测试

```bash
# 克隆仓库
git clone https://github.com/Summe-rsnow/Bento.git
cd Bento

# 运行单元测试（SemVer 更新检测、拖拽槽位计算、shell 状态键往返等）
cargo test
```

---

## 📁 项目结构

```text
Bento/
├── assets/                 # 品牌 Logo 与向量标志（logo.svg / logo-mark.svg / icon.png）
├── Cargo.toml              # Crate 元信息与依赖配置（v1.0.0，Rust 2024）
├── LICENSE                 # Apache License 2.0 开源协议
├── README.md               # 项目说明文档
├── theme.json              # 家族统一浅色 / 深色语义色 Token（DEFAULT_THEME_JSON）
└── src/
    ├── lib.rs              # 库根入口：导出子模块、AppMeta 与 DEFAULT_THEME_JSON
    ├── drag_list.rs        # 垂直卡片列表拖拽排序控制器与槽位几何计算
    ├── shell.rs            # 隐藏标题栏、外观切换、关闭策略、托盘子菜单、NoticeCenter 与本地打开
    ├── updater.rs          # AppMeta、GitHub Releases SemVer 更新检测与共用关于弹窗
    └── ui/
        ├── mod.rs          # UI 层统一导出、ValueTone、focus_ring、mono_font 与动效常量
        ├── button.rs       # 按钮、图标按钮、分段图标控件、提示气泡与外链
        ├── icons.rs        # 统一家族 24×24 描边图标库、brand_mark_svg 与 png_data_uri
        ├── input.rs        # 文本输入框、多行文本框、搜索框、复选框、开关与单选框
        ├── nav.rs          # 侧边栏导航项（带数量徽标）与过滤芯片
        ├── progress.rs     # 线性进度条
        └── surface.rs      # 卡片、分隔线、状态徽标、悬浮提示、模态弹窗、meta_row 与 confirm_dialog
```

---

## 🤝 参与贡献

欢迎提交 Issue 或 Pull Request 来完善 Bento！  
提交前请确保 `cargo test` 全部通过且无编译警告，新增控件遵循 `theme()` 语义色 Token 与 `focus_visible` 无障碍规范。

## 📄 开源协议

本项目基于 [Apache License 2.0](LICENSE) 开源。
