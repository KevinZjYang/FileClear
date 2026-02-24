"""Main window for FileClear application - Dark Theme Edition."""
import os
import datetime
from PyQt5.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QTableWidget, QTableWidgetItem, QHeaderView,
    QLabel, QFileDialog, QMessageBox, QProgressBar, QSplitter,
    QFrame, QAbstractItemView, QScrollArea, QStackedWidget
)
from PyQt5.QtCore import Qt, QThread, pyqtSignal, QSize
from PyQt5.QtGui import QFont, QColor, QIcon, QPalette, QBrush

from core import MetadataCleaner


# Color scheme - Light theme
COLOR_BG = "#f5f5f5"
COLOR_BG_LIGHT = "#ffffff"
COLOR_BG_PANEL = "#ffffff"
COLOR_BG_HOVER = "#e8e8e8"
COLOR_ACCENT = "#0078d4"
COLOR_ACCENT_HOVER = "#106ebe"
COLOR_TEXT = "#333333"
COLOR_TEXT_DIM = "#666666"
COLOR_BORDER = "#d0d0d0"
COLOR_SUCCESS = "#16c60c"
COLOR_ERROR = "#e81123"


def get_file_icon(file_type: str) -> str:
    """Get icon character for file type."""
    icons = {
        'JPEG Image': '🖼️',
        'PNG Image': '🖼️',
        'GIF Image': '🖼️',
        'WebP Image': '🖼️',
        'TIFF Image': '🖼️',
        'BMP Image': '🖼️',
        'PDF Document': '📄',
        'Word Document': '📝',
        'Word Document (Legacy)': '📝',
        'Excel Spreadsheet': '📊',
        'Excel Spreadsheet (Legacy)': '📊',
        'PowerPoint Presentation': '📽️',
        'PowerPoint Presentation (Legacy)': '📽️',
    }
    return icons.get(file_type, '📁')


def format_size(size: int) -> str:
    """Format file size."""
    for unit in ['B', 'KB', 'MB', 'GB']:
        if size < 1024:
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} TB"


class CleanWorker(QThread):
    """Worker thread for cleaning files."""
    progress = pyqtSignal(int, int, str)
    finished = pyqtSignal(str, str, bool, str)
    all_finished = pyqtSignal(int, int)

    def __init__(self, files: list):
        super().__init__()
        self.files = files

    def run(self):
        success_count = 0
        total = len(self.files)

        for i, file_path in enumerate(self.files):
            filename = os.path.basename(file_path)
            self.progress.emit(i + 1, total, filename)

            try:
                cleaned_path = MetadataCleaner.clean(file_path)
                success = os.path.exists(cleaned_path)
                if success:
                    success_count += 1
                self.finished.emit(file_path, cleaned_path, success, '')
            except Exception as e:
                self.finished.emit(file_path, '', False, str(e))

        self.all_finished.emit(success_count, total)


class FileClearMainWindow(QMainWindow):
    """Main window for FileClear application - Dark Theme."""

    def __init__(self):
        super().__init__()
        self.files = []
        self.file_data = {}  # Store file metadata
        self.worker = None
        self.init_ui()

    def init_ui(self):
        """Initialize the dark theme user interface."""
        self.setWindowTitle('FileClear - 文件元数据清洗')
        self.setGeometry(100, 100, 1000, 700)
        self.setMinimumSize(800, 600)

        # Set dark palette
        self.set_light_palette()

        # Central widget
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        # Main layout
        main_layout = QVBoxLayout(central_widget)
        main_layout.setContentsMargins(20, 20, 20, 20)
        main_layout.setSpacing(15)

        # Header
        self.create_header(main_layout)

        # Content area
        self.create_content(main_layout)

        # Bottom bar
        self.create_bottom_bar(main_layout)

    def set_light_palette(self):
        """Set light theme palette."""
        palette = QPalette()
        palette.setColor(QPalette.Window, QColor(COLOR_BG))
        palette.setColor(QPalette.WindowText, QColor(COLOR_TEXT))
        palette.setColor(QPalette.Base, QColor(COLOR_BG_LIGHT))
        palette.setColor(QPalette.AlternateBase, QColor(COLOR_BG_LIGHT))
        palette.setColor(QPalette.ToolTipBase, QColor(COLOR_BG_LIGHT))
        palette.setColor(QPalette.ToolTipText, QColor(COLOR_TEXT))
        palette.setColor(QPalette.Text, QColor(COLOR_TEXT))
        palette.setColor(QPalette.Button, QColor(COLOR_BG_LIGHT))
        palette.setColor(QPalette.ButtonText, QColor(COLOR_TEXT))
        palette.setColor(QPalette.BrightText, QColor(COLOR_TEXT))
        palette.setColor(QPalette.Link, QColor(COLOR_ACCENT))
        palette.setColor(QPalette.Highlight, QColor(COLOR_ACCENT))
        palette.setColor(QPalette.HighlightedText, QColor(COLOR_TEXT))
        self.setPalette(palette)

    def create_header(self, parent_layout):
        """Create header section."""
        header_widget = QWidget()
        header_layout = QHBoxLayout(header_widget)
        header_layout.setContentsMargins(0, 0, 0, 0)

        # Logo and title
        title_label = QLabel("🧹 FileClear")
        title_font = QFont("Microsoft YaHei", 20, QFont.Bold)
        title_label.setFont(title_font)
        title_label.setStyleSheet(f"color: {COLOR_TEXT};")
        header_layout.addWidget(title_label)

        header_layout.addStretch()

        # Subtitle
        subtitle = QLabel("文件元数据清洗工具")
        subtitle.setFont(QFont("Microsoft YaHei", 10))
        subtitle.setStyleSheet(f"color: {COLOR_TEXT_DIM};")
        header_layout.addWidget(subtitle)

        parent_layout.addWidget(header_widget)

    def create_content(self, parent_layout):
        """Create main content area with splitter."""
        # Drop zone hint
        hint_label = QLabel("👆 拖拽文件或文件夹到下方列表")
        hint_label.setFont(QFont("Microsoft YaHei", 10))
        hint_label.setStyleSheet(f"color: {COLOR_TEXT_DIM}; padding: 5px;")
        hint_label.setAlignment(Qt.AlignCenter)
        parent_layout.addWidget(hint_label)

        # Main splitter
        splitter = QSplitter(Qt.Horizontal)

        # Left: File table
        self.file_table = QTableWidget()
        self.file_table.setColumnCount(4)
        self.file_table.setHorizontalHeaderLabels(["", "文件名", "大小", "修改时间"])
        self.file_table.setSelectionBehavior(QAbstractItemView.SelectRows)
        self.file_table.setSelectionMode(QAbstractItemView.ExtendedSelection)
        self.file_table.setShowGrid(False)
        self.file_table.setAlternatingRowColors(True)
        self.file_table.horizontalHeader().setStretchLastSection(True)
        self.file_table.horizontalHeader().setSectionResizeMode(1, QHeaderView.Stretch)
        self.file_table.verticalHeader().setVisible(False)
        self.file_table.setAcceptDrops(True)

        # Style the table
        self.file_table.setStyleSheet(f"""
            QTableWidget {{
                background-color: {COLOR_BG_LIGHT};
                border: 1px solid {COLOR_BORDER};
                border-radius: 8px;
                padding: 5px;
            }}
            QTableWidget::item {{
                padding: 8px;
                border-bottom: 1px solid {COLOR_BG};
                color: {COLOR_TEXT};
            }}
            QTableWidget::item:selected {{
                background-color: {COLOR_ACCENT};
            }}
            QTableWidget::item:hover {{
                background-color: {COLOR_BG_HOVER};
            }}
            QHeaderView::section {{
                background-color: {COLOR_BG};
                color: {COLOR_TEXT_DIM};
                padding: 8px;
                border: none;
                font-weight: bold;
            }}
            QTableWidget QTableCornerButton::section {{
                background-color: {COLOR_BG};
                border: none;
            }}
        """)

        splitter.addWidget(self.file_table)

        # Right: Metadata panel
        self.metadata_panel = self.create_metadata_panel()
        splitter.addWidget(self.metadata_panel)

        # Set splitter proportions
        splitter.setSizes([500, 300])

        parent_layout.addWidget(splitter, 1)

        # Connect signals
        self.file_table.itemClicked.connect(self.on_table_item_clicked)

        # Setup drag and drop for file table
        self.file_table.dragEnterEvent = self.dragEnterEvent
        self.file_table.dragMoveEvent = self.dragMoveEvent
        self.file_table.dropEvent = self.dropEvent

    def create_metadata_panel(self) -> QWidget:
        """Create metadata display panel."""
        panel = QFrame()
        panel.setFrameShape(QFrame.StyledPanel)
        panel.setStyleSheet(f"""
            QFrame {{
                background-color: {COLOR_BG_LIGHT};
                border: 1px solid {COLOR_BORDER};
                border-radius: 8px;
            }}
        """)

        layout = QVBoxLayout(panel)
        layout.setContentsMargins(15, 15, 15, 15)
        layout.setSpacing(10)

        # Title
        title = QLabel("📋 元数据信息")
        title.setFont(QFont("Microsoft YaHei", 12, QFont.Bold))
        title.setStyleSheet(f"color: {COLOR_TEXT};")
        layout.addWidget(title)

        # Content area
        self.metadata_content = QWidget()
        self.metadata_content.setStyleSheet(f"color: {COLOR_TEXT};")
        self.metadata_layout = QVBoxLayout(self.metadata_content)
        self.metadata_layout.setContentsMargins(0, 10, 0, 0)
        self.metadata_layout.setSpacing(8)

        # Scroll area
        scroll = QScrollArea()
        scroll.setWidget(self.metadata_content)
        scroll.setWidgetResizable(True)
        scroll.setStyleSheet(f"""
            QScrollArea {{
                border: none;
                background-color: transparent;
            }}
            QScrollBar:vertical {{
                background: {COLOR_BG};
                width: 8px;
                border-radius: 4px;
            }}
            QScrollBar::handle:vertical {{
                background: {COLOR_BORDER};
                border-radius: 4px;
            }}
            QScrollBar::handle:vertical:hover {{
                background: {COLOR_ACCENT};
            }}
        """)
        layout.addWidget(scroll)

        # Placeholder
        self.set_metadata_placeholder()

        return panel

    def set_metadata_placeholder(self):
        """Set placeholder text when no file is selected."""
        # Clear existing widgets
        while self.metadata_layout.count():
            item = self.metadata_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        placeholder = QLabel("选择文件查看元数据")
        placeholder.setFont(QFont("Microsoft YaHei", 10))
        placeholder.setStyleSheet(f"color: {COLOR_TEXT_DIM};")
        placeholder.setAlignment(Qt.AlignCenter)
        self.metadata_layout.addWidget(placeholder)

    def display_metadata(self, file_path: str, metadata: dict):
        """Display metadata for a file."""
        # Clear existing widgets
        while self.metadata_layout.count():
            item = self.metadata_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        # File info card
        file_type = MetadataCleaner.get_file_type(file_path)
        icon = get_file_icon(file_type)

        # Header with icon
        header = QWidget()
        header_layout = QHBoxLayout(header)
        header_layout.setContentsMargins(0, 0, 0, 0)

        icon_label = QLabel(f"{icon}  {os.path.basename(file_path)}")
        icon_label.setFont(QFont("Microsoft YaHei", 12, QFont.Bold))
        icon_label.setStyleSheet(f"color: {COLOR_TEXT};")
        header_layout.addWidget(icon_label)

        self.metadata_layout.addWidget(header)

        # Separator
        sep = QFrame()
        sep.setFrameShape(QFrame.HLine)
        sep.setStyleSheet(f"background-color: {COLOR_BORDER}; max-height: 1px;")
        self.metadata_layout.addWidget(sep)

        # Metadata items
        if metadata:
            for key, value in metadata.items():
                if key != 'error':
                    self.add_metadata_item(key, str(value))
        else:
            no_data = QLabel("未发现元数据")
            no_data.setFont(QFont("Microsoft YaHei", 10))
            no_data.setStyleSheet(f"color: {COLOR_SUCCESS};")
            self.metadata_layout.addWidget(no_data)

        if 'error' in metadata:
            error = QLabel(f"错误: {metadata['error']}")
            error.setFont(QFont("Microsoft YaHei", 9))
            error.setStyleSheet(f"color: {COLOR_ERROR};")
            self.metadata_layout.addWidget(error)

        self.metadata_layout.addStretch()

    def add_metadata_item(self, key: str, value: str):
        """Add a metadata item to the panel."""
        widget = QWidget()
        layout = QHBoxLayout(widget)
        layout.setContentsMargins(0, 3, 0, 3)

        key_label = QLabel(f"{key}:")
        key_label.setFont(QFont("Microsoft YaHei", 9, QFont.Bold))
        key_label.setStyleSheet(f"color: {COLOR_ACCENT}; min-width: 100px;")
        key_label.setAlignment(Qt.AlignRight)
        layout.addWidget(key_label)

        value_label = QLabel(value)
        value_label.setFont(QFont("Microsoft YaHei", 9))
        value_label.setStyleSheet(f"color: {COLOR_TEXT};")
        value_label.setWordWrap(True)
        layout.addWidget(value_label, 1)

        self.metadata_layout.addWidget(widget)

    def create_bottom_bar(self, parent_layout):
        """Create bottom action bar."""
        bar = QWidget()
        bar_layout = QHBoxLayout(bar)
        bar_layout.setContentsMargins(0, 10, 0, 0)

        # Left buttons
        btn_group = QWidget()
        btn_layout = QHBoxLayout(btn_group)
        btn_layout.setSpacing(10)

        add_file_btn = self.create_button("➕ 添加文件")
        add_file_btn.clicked.connect(self.add_files)
        btn_layout.addWidget(add_file_btn)

        add_folder_btn = self.create_button("📁 添加文件夹")
        add_folder_btn.clicked.connect(self.add_folder)
        btn_layout.addWidget(add_folder_btn)

        clear_btn = self.create_button("🗑️ 清空")
        clear_btn.clicked.connect(self.clear_list)
        btn_layout.addWidget(clear_btn)

        bar_layout.addWidget(btn_group)

        bar_layout.addStretch()

        # Status
        self.status_label = QLabel("就绪")
        self.status_label.setFont(QFont("Microsoft YaHei", 9))
        self.status_label.setStyleSheet(f"color: {COLOR_TEXT_DIM};")
        bar_layout.addWidget(self.status_label)

        # Progress bar
        self.progress_bar = QProgressBar()
        self.progress_bar.setFixedWidth(200)
        self.progress_bar.setVisible(False)
        self.progress_bar.setStyleSheet(f"""
            QProgressBar {{
                background-color: {COLOR_BG_LIGHT};
                border: 1px solid {COLOR_BORDER};
                border-radius: 4px;
                text-align: center;
                color: {COLOR_TEXT};
            }}
            QProgressBar::chunk {{
                background-color: {COLOR_ACCENT};
                border-radius: 3px;
            }}
        """)
        bar_layout.addWidget(self.progress_bar)

        # Clean button
        self.clean_btn = self.create_button("✨ 开始清洗", primary=True)
        self.clean_btn.clicked.connect(self.start_cleaning)
        self.clean_btn.setEnabled(False)
        bar_layout.addWidget(self.clean_btn)

        parent_layout.addWidget(bar)

    def create_button(self, text: str, primary: bool = False) -> QPushButton:
        """Create a styled button."""
        btn = QPushButton(text)
        btn.setFont(QFont("Microsoft YaHei", 10))

        if primary:
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: {COLOR_ACCENT};
                    color: white;
                    border: none;
                    padding: 10px 25px;
                    border-radius: 6px;
                    font-weight: bold;
                }}
                QPushButton:hover {{
                    background-color: {COLOR_ACCENT_HOVER};
                }}
                QPushButton:disabled {{
                    background-color: {COLOR_BORDER};
                    color: {COLOR_TEXT_DIM};
                }}
            """)
        else:
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: {COLOR_BG_LIGHT};
                    color: {COLOR_TEXT};
                    border: 1px solid {COLOR_BORDER};
                    padding: 8px 16px;
                    border-radius: 6px;
                }}
                QPushButton:hover {{
                    background-color: {COLOR_BG_HOVER};
                    border-color: {COLOR_ACCENT};
                }}
            """)

        return btn

    # Event handlers
    def dragEnterEvent(self, event):
        """Handle drag enter."""
        if event.mimeData().hasUrls():
            event.acceptProposedAction()

    def dragMoveEvent(self, event):
        """Handle drag move."""
        if event.mimeData().hasUrls():
            event.acceptProposedAction()

    def dropEvent(self, event):
        """Handle file drop."""
        if event.mimeData().hasUrls():
            for url in event.mimeData().urls():
                path = url.toLocalFile()
                if os.path.isfile(path):
                    self.add_file(path)
                elif os.path.isdir(path):
                    self.add_files_from_folder(path)

    def on_table_item_clicked(self, item):
        """Handle table item click."""
        row = item.row()
        if 0 <= row < len(self.files):
            file_path = self.files[row]
            try:
                metadata = MetadataCleaner.read_metadata(file_path)
                self.display_metadata(file_path, metadata)
            except Exception as e:
                self.display_metadata(file_path, {'error': str(e)})

    def add_file(self, file_path: str):
        """Add a single file to the list."""
        if MetadataCleaner.is_supported(file_path):
            if file_path not in self.files:
                self.files.append(file_path)
                self.add_file_to_table(file_path)
                self.update_buttons()
        else:
            QMessageBox.warning(
                self,
                '不支持的文件类型',
                f'不支持: {os.path.splitext(file_path)[1]}'
            )

    def add_file_to_table(self, file_path: str):
        """Add file to the table widget."""
        row = self.file_table.rowCount()
        self.file_table.insertRow(row)

        # Get file info
        stat = os.stat(file_path)
        file_type = MetadataCleaner.get_file_type(file_path)
        icon = get_file_icon(file_type)
        size = format_size(stat.st_size)
        mtime = datetime.datetime.fromtimestamp(stat.st_mtime).strftime("%Y-%m-%d %H:%M")

        # Icon column
        icon_item = QTableWidgetItem(icon)
        icon_item.setTextAlignment(Qt.AlignCenter)
        icon_item.setFont(QFont("Microsoft YaHei", 14))
        self.file_table.setItem(row, 0, icon_item)

        # Filename column
        name_item = QTableWidgetItem(os.path.basename(file_path))
        name_item.setFont(QFont("Microsoft YaHei", 10))
        self.file_table.setItem(row, 1, name_item)

        # Size column
        size_item = QTableWidgetItem(size)
        size_item.setFont(QFont("Microsoft YaHei", 9))
        size_item.setForeground(QColor(COLOR_TEXT_DIM))
        size_item.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
        self.file_table.setItem(row, 2, size_item)

        # Time column
        time_item = QTableWidgetItem(mtime)
        time_item.setFont(QFont("Microsoft YaHei", 9))
        time_item.setForeground(QColor(COLOR_TEXT_DIM))
        time_item.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
        self.file_table.setItem(row, 3, time_item)

    def add_files(self):
        """Open file dialog to add files."""
        files, _ = QFileDialog.getOpenFileNames(
            self,
            '选择文件',
            '',
            '所有支持的文件 (*.jpg *.jpeg *.png *.gif *.webp *.tiff *.tif *.bmp *.pdf *.docx *.doc *.xlsx *.xls *.pptx *.ppt);;'
            '图片文件 (*.jpg *.jpeg *.png *.gif *.webp *.tiff *.tif *.bmp);;'
            'PDF文件 (*.pdf);;'
            'Word文档 (*.docx *.doc);;'
            'Excel表格 (*.xlsx *.xls);;'
            'PowerPoint (*.pptx *.ppt)'
        )

        for file_path in files:
            self.add_file(file_path)

    def add_folder(self):
        """Open folder dialog to add files."""
        folder = QFileDialog.getExistingDirectory(self, '选择文件夹')
        if folder:
            self.add_files_from_folder(folder)

    def add_files_from_folder(self, folder: str):
        """Add all supported files from a folder."""
        for root, dirs, files in os.walk(folder):
            for file in files:
                file_path = os.path.join(root, file)
                self.add_file(file_path)

    def clear_list(self):
        """Clear the file list."""
        self.files.clear()
        self.file_table.setRowCount(0)
        self.set_metadata_placeholder()
        self.update_buttons()

    def update_buttons(self):
        """Update button states."""
        has_files = len(self.files) > 0
        self.clean_btn.setEnabled(has_files)
        self.status_label.setText(f"已选择 {len(self.files)} 个文件")

    def start_cleaning(self):
        """Start cleaning metadata for all files."""
        if not self.files:
            return

        # Disable buttons
        self.clean_btn.setEnabled(False)
        self.progress_bar.setVisible(True)
        self.progress_bar.setMaximum(len(self.files))
        self.progress_bar.setValue(0)

        # Start worker
        self.worker = CleanWorker(self.files)
        self.worker.progress.connect(self.on_progress)
        self.worker.finished.connect(self.on_file_finished)
        self.worker.all_finished.connect(self.on_all_finished)
        self.worker.start()

    def on_progress(self, current: int, total: int, filename: str):
        """Handle progress update."""
        self.progress_bar.setValue(current)
        self.status_label.setText(f"正在处理: {filename} ({current}/{total})")

    def on_file_finished(self, original: str, cleaned: str, success: bool, error: str):
        """Handle single file finished."""
        row = self.files.index(original)

        if success:
            # Mark as cleaned in table
            item = self.file_table.item(row, 1)
            item.setText(f"✓ {os.path.basename(original)}")
            item.setForeground(QColor(COLOR_SUCCESS))
        else:
            item = self.file_table.item(row, 1)
            item.setText(f"✗ {os.path.basename(original)}")
            item.setForeground(QColor(COLOR_ERROR))

    def on_all_finished(self, success_count: int, total_count: int):
        """Handle all files finished."""
        self.status_label.setText(f"完成! 成功: {success_count}/{total_count}")
        self.clean_btn.setEnabled(True)
        self.progress_bar.setVisible(False)

        QMessageBox.information(
            self,
            '清洗完成',
            f"成功清洗 {success_count}/{total_count} 个文件"
        )
