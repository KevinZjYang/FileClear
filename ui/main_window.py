"""Main window for FileClear application."""
import os
import sys
from PyQt5.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QListWidget, QLabel, QFileDialog,
    QMessageBox, QProgressBar, QTextEdit, QSplitter,
    QGroupBox, QFrame
)
from PyQt5.QtCore import Qt, QThread, pyqtSignal
from PyQt5.QtGui import QIcon, QFont

from core import MetadataCleaner


class CleanWorker(QThread):
    """Worker thread for cleaning files."""
    progress = pyqtSignal(int, int, str)  # current, total, filename
    finished = pyqtSignal(str, str, bool, str)  # original, cleaned, success, error
    all_finished = pyqtSignal(int, int)  # success_count, total_count

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
    """Main window for FileClear application."""

    def __init__(self):
        super().__init__()
        self.files = []
        self.worker = None
        self.init_ui()

    def init_ui(self):
        """Initialize the user interface."""
        self.setWindowTitle('FileClear - 文件元数据清洗工具')
        self.setGeometry(100, 100, 900, 650)

        # Central widget
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        # Main layout
        main_layout = QVBoxLayout(central_widget)

        # Header
        header_label = QLabel('文件元数据清洗工具')
        header_label.setFont(QFont('Microsoft YaHei', 16, QFont.Bold))
        main_layout.addWidget(header_label)

        # Description
        desc_label = QLabel('拖拽文件到下方列表，或点击按钮选择文件')
        desc_label.setStyleSheet('color: #666;')
        main_layout.addWidget(desc_label)

        # Splitter for file list and metadata
        splitter = QSplitter(Qt.Horizontal)

        # File list group
        file_group = QGroupBox('文件列表')
        file_layout = QVBoxLayout()

        self.file_list = QListWidget()
        self.file_list.setAcceptDrops(True)
        self.file_list.setMinimumWidth(300)
        self.file_list.setDragDropMode(QListWidget.DragDrop)
        self.file_list.setSelectionMode(QListWidget.ExtendedSelection)
        file_layout.addWidget(self.file_list)

        # Button layout
        btn_layout = QHBoxLayout()

        self.add_btn = QPushButton('添加文件')
        self.add_btn.clicked.connect(self.add_files)
        btn_layout.addWidget(self.add_btn)

        self.add_folder_btn = QPushButton('添加文件夹')
        self.add_folder_btn.clicked.connect(self.add_folder)
        btn_layout.addWidget(self.add_folder_btn)

        self.clear_btn = QPushButton('清空列表')
        self.clear_btn.clicked.connect(self.clear_list)
        btn_layout.addWidget(self.clear_btn)

        file_layout.addLayout(btn_layout)
        file_group.setLayout(file_layout)
        splitter.addWidget(file_group)

        # Metadata display group
        metadata_group = QGroupBox('元数据信息')
        metadata_layout = QVBoxLayout()

        self.metadata_display = QTextEdit()
        self.metadata_display.setReadOnly(True)
        self.metadata_display.setPlaceholderText('选择文件查看元数据信息...')
        metadata_layout.addWidget(self.metadata_display)

        metadata_group.setLayout(metadata_layout)
        splitter.addWidget(metadata_group)

        main_layout.addWidget(splitter)

        # Progress bar
        self.progress_bar = QProgressBar()
        self.progress_bar.setVisible(False)
        main_layout.addWidget(self.progress_bar)

        # Status and action buttons
        bottom_layout = QHBoxLayout()

        self.status_label = QLabel('就绪')
        bottom_layout.addWidget(self.status_label)

        bottom_layout.addStretch()

        self.clean_btn = QPushButton('开始清洗')
        self.clean_btn.clicked.connect(self.start_cleaning)
        self.clean_btn.setEnabled(False)
        bottom_layout.addWidget(self.clean_btn)

        main_layout.addLayout(bottom_layout)

        # Connect signals
        self.file_list.itemClicked.connect(self.show_metadata)

        # Set drag and drop event filters
        self.file_list.dragEnterEvent = self.drag_enter_event
        self.file_list.dragMoveEvent = self.drag_move_event
        self.file_list.dropEvent = self.drop_event

    def drag_enter_event(self, event):
        """Handle drag enter event."""
        if event.mimeData().hasUrls():
            event.accept()
        else:
            event.ignore()

    def drag_move_event(self, event):
        """Handle drag move event."""
        if event.mimeData().hasUrls():
            event.accept()
        else:
            event.ignore()

    def drop_event(self, event):
        """Handle drop event."""
        if event.mimeData().hasUrls():
            for url in event.mimeData().urls():
                path = url.toLocalFile()
                if os.path.isfile(path):
                    self.add_file(path)
                elif os.path.isdir(path):
                    self.add_files_from_folder(path)

    def add_file(self, file_path: str):
        """Add a single file to the list."""
        if MetadataCleaner.is_supported(file_path):
            if file_path not in self.files:
                self.files.append(file_path)
                self.file_list.addItem(os.path.basename(file_path))
                self.update_buttons()
        else:
            QMessageBox.warning(
                self,
                '不支持的文件类型',
                f'不支持的文件类型: {os.path.splitext(file_path)[1]}'
            )

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
        """Open folder dialog to add all supported files from folder."""
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
        self.file_list.clear()
        self.metadata_display.clear()
        self.update_buttons()

    def show_metadata(self, item):
        """Show metadata for selected file."""
        index = self.file_list.row(item)
        if 0 <= index < len(self.files):
            file_path = self.files[index]
            try:
                metadata = MetadataCleaner.read_metadata(file_path)

                # Build metadata display
                display = f"文件: {os.path.basename(file_path)}\n"
                display += f"类型: {MetadataCleaner.get_file_type(file_path)}\n"
                display += f"路径: {file_path}\n\n"
                display += "元数据:\n"
                display += "-" * 40 + "\n"

                if metadata:
                    for key, value in metadata.items():
                        if key != 'error':
                            display += f"{key}: {value}\n"
                else:
                    display += "(未发现元数据)\n"

                if 'error' in metadata:
                    display += f"\n错误: {metadata['error']}\n"

                self.metadata_display.setPlainText(display)

            except Exception as e:
                self.metadata_display.setPlainText(f"读取元数据失败: {str(e)}")

    def update_buttons(self):
        """Update button states based on file list."""
        has_files = len(self.files) > 0
        self.clean_btn.setEnabled(has_files)
        self.status_label.setText(f'已选择 {len(self.files)} 个文件')

    def start_cleaning(self):
        """Start cleaning metadata for all files."""
        if not self.files:
            return

        # Disable buttons during processing
        self.add_btn.setEnabled(False)
        self.add_folder_btn.setEnabled(False)
        self.clear_btn.setEnabled(False)
        self.clean_btn.setEnabled(False)
        self.progress_bar.setVisible(True)
        self.progress_bar.setMaximum(len(self.files))
        self.progress_bar.setValue(0)

        # Start worker thread
        self.worker = CleanWorker(self.files)
        self.worker.progress.connect(self.on_progress)
        self.worker.finished.connect(self.on_file_finished)
        self.worker.all_finished.connect(self.on_all_finished)
        self.worker.start()

    def on_progress(self, current: int, total: int, filename: str):
        """Handle progress update."""
        self.progress_bar.setValue(current)
        self.status_label.setText(f'正在处理: {filename} ({current}/{total})')

    def on_file_finished(self, original: str, cleaned: str, success: bool, error: str):
        """Handle single file finished."""
        if success:
            index = self.files.index(original)
            item = self.file_list.item(index)
            item.setText(f'✓ {os.path.basename(original)} -> {os.path.basename(cleaned)}')
        else:
            index = self.files.index(original)
            item = self.file_list.item(index)
            item.setText(f'✗ {os.path.basename(original)} - {error}')

    def on_all_finished(self, success_count: int, total_count: int):
        """Handle all files finished."""
        self.status_label.setText(f'完成! 成功: {success_count}/{total_count}')

        # Re-enable buttons
        self.add_btn.setEnabled(True)
        self.add_folder_btn.setEnabled(True)
        self.clear_btn.setEnabled(True)
        self.clean_btn.setEnabled(True)

        QMessageBox.information(
            self,
            '清洗完成',
            f'成功清洗 {success_count}/{total_count} 个文件'
        )
