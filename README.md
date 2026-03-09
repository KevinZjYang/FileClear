# FileClear(元数据清洁器 )

一个简单易用的桌面应用程序，用于清除各种文件中的元数据，保护您的隐私。

## 功能特点

- 支持多种文件格式：
  - 图片：JPEG, PNG, GIF, WebP, TIFF, BMP
  - 文档：PDF, Word (DOCX/DOC)
  - 表格：Excel (XLSX/XLS)
  - 演示文稿：PowerPoint (PPTX/PPT)

- 便捷操作：
  - 拖拽文件或文件夹添加
  - 批量处理多个文件
  - 实时显示文件元数据
  - 清洗进度可视化

- 用户友好：
  - 简洁直观的界面
  - 支持中文显示
  - 清洗结果清晰可见

## 环境要求

- Python 3.8+
- Windows 10/11

## 安装步骤

1. 克隆或下载本项目

2. 安装依赖：
   ```
   pip install -r requirements.txt
   ```

3. 运行程序：
   ```
   python main.py
   ```

## 使用方法

1. 启动程序后，可以通过以下方式添加文件：
   - 点击"添加文件"按钮选择单个或多个文件
   - 点击"添加文件夹"选择整个文件夹
   - 直接拖拽文件或文件夹到列表区域

2. 添加文件后，可以在右侧面板查看文件的元数据信息

3. 点击"开始清洗"按钮，程序将处理所有添加的文件

4. 清洗完成后，原文件将被清洗后的新文件替换

## 项目结构

```
FileClear/
├── main.py                 # 程序入口
├── requirements.txt        # 依赖列表
├── core/                   # 核心功能模块
│   ├── cleaner.py          # 清洗器主类
│   └── cleaners/           # 各类文件清洗器
│       ├── excel_cleaner.py
│       ├── image_cleaner.py
│       ├── pdf_cleaner.py
│       ├── ppt_cleaner.py
│       └── word_cleaner.py
└── ui/                     # UI模块
    └── main_window.py      # 主窗口
```

## 许可证

MIT License
