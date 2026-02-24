"""FileClear - 文件元数据清洗工具

Main entry point for the application.
"""
import sys
from PyQt5.QtWidgets import QApplication
from ui.main_window import FileClearMainWindow


def main():
    """Main function to start the application."""
    app = QApplication(sys.argv)
    app.setStyle('Fusion')  # Modern look

    window = FileClearMainWindow()
    window.show()

    sys.exit(app.exec_())


if __name__ == '__main__':
    main()
