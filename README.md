# FileClear（元数据清洁器）

一个基于 **Tauri v2 + Vue 3** 的桌面应用，用于清除各类文件中的元数据，保护您的隐私。支持在 Windows 资源管理器中通过**右键菜单一键快捷清理**。

## 功能特点

- 支持的格式：
  - 图片：JPEG、PNG、GIF、WebP、TIFF、BMP
  - 文档：PDF、Word（DOCX/DOC）
  - 表格：Excel（XLSX/XLS）
  - 演示文稿：PowerPoint（PPTX/PPT）
- 清洗方式：
  - 图片：无损剥离 EXIF/XMP/ICC/Photoshop 等元数据段，GIF 保留动画帧与延时
  - PDF：清空 Info 字典并移除 XMP 元数据
  - Office 新格式（docx/xlsx/pptx）：清空 docProps 元数据 XML
  - 旧格式（doc/xls/ppt）：覆写 OLE SummaryInformation 摘要流为最小合法空 PropertySet
- 便捷操作：
  - 拖拽或选择文件/文件夹添加
  - 批量清洗、逐文件进度可视化
  - 元数据预览抽屉
  - 资源管理器右键菜单"用 FileClear 清理元数据"，支持单/多文件一键快捷清理，完成后弹出系统通知
- 用户友好：中文界面，清洗结果清晰可见

## 技术栈

- 桌面框架：Tauri v2（Rust 后端）
- 前端：Vue 3 + TypeScript + Vite
- 组件库：Element Plus

## 环境要求

- Windows 10/11
- Node.js 18+ 与 [pnpm](https://pnpm.io/)
- [Rust 工具链](https://rustup.rs/)（含 MSVC 构建工具）
- 可选：WebView2（Windows 10/11 一般已内置）

## 开发

```bash
# 安装前端依赖（pnpm 会按 pnpm-workspace.yaml 放行 esbuild 构建脚本）
pnpm install

# 启动开发模式（打开带热更新的桌面窗口）
pnpm tauri dev

# 前端类型检查与构建
pnpm build

# 运行 Rust 单元测试（清洗引擎）
cd src-tauri && cargo test

# 打包安装程序（NSIS）
pnpm tauri build
```

## 使用方法

1. 启动后可通过"添加文件/添加文件夹"或直接拖拽，将文件加入列表。
2. 选中文件可查看元数据预览。
3. 点击"开始清洗"，原文件将被**原位覆盖**为清洗后的文件。
4. 在"设置"中可开关"资源管理器右键菜单"：
   - 开启后注册 `HKCU\Software\Classes\*\shell\FileClear`（仅当前用户，无需管理员权限）。
   - 在资源管理器中右键文件（可多选）→ "用 FileClear 清理元数据"即可快捷清理。
   - Windows 11 中该菜单项位于"显示更多选项"二级菜单。
   - 关闭开关会移除注册表项；首次启动会自动注册。

## 项目结构

```
FileClear/
├── src/                        # Vue 3 前端
│   ├── App.vue                 # 主界面
│   ├── api.ts                  # Tauri 命令封装
│   └── types.ts                # 前端类型定义
├── src-tauri/                  # Tauri / Rust 后端
│   ├── src/
│   │   ├── main.rs             # 入口，解析 --quick-clean 参数
│   │   ├── lib.rs              # 应用装配、单实例转发
│   │   ├── commands.rs         # 前端调用的 Tauri 命令
│   │   ├── cleaners/           # 清洗引擎
│   │   │   ├── image.rs        # 图片元数据剥离
│   │   │   ├── pdf.rs          # PDF 清理
│   │   │   ├── ooxml.rs        # Office 新格式清理
│   │   │   ├── legacy.rs       # 旧格式 OLE 摘要流覆写
│   │   │   └── propset.rs      # MS-OLEPS PropertySet 解析/生成
│   │   ├── metadata.rs         # 元数据读取
│   │   ├── quick_clean.rs      # 右键菜单快捷清理
│   │   ├── context_menu.rs     # 右键菜单注册/移除（HKCU）
│   │   └── settings.rs         # 设置持久化
│   ├── capabilities/           # Tauri 权限配置
│   └── tauri.conf.json         # 应用配置（窗口默认隐藏，普通启动时显示）
└── package.json
```

## 已知限制

- 原位覆盖暂不提供回收站备份（可在后续版本增加设置项）。
- 动画 WebP 清洗时仅保留第一帧（编码器限制），其余格式动画保持完整。
- 加密 PDF 暂不支持清理。
- 打包安装后的系统通知需在安装版中验证（`tauri dev` 下可能不显示）。

## 许可证

MIT License