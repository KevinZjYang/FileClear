"""Unified metadata cleaner interface."""
import os
from typing import Optional, Dict, List
from .cleaners import (
    ImageCleaner,
    PDFCleaner,
    WordCleaner,
    ExcelCleaner,
    PPTCleaner,
)


# Map file extensions to cleaner classes
CLEANER_MAP = {
    # Images
    '.jpg': ImageCleaner,
    '.jpeg': ImageCleaner,
    '.png': ImageCleaner,
    '.gif': ImageCleaner,
    '.webp': ImageCleaner,
    '.tiff': ImageCleaner,
    '.tif': ImageCleaner,
    '.bmp': ImageCleaner,
    # PDF
    '.pdf': PDFCleaner,
    # Office
    '.docx': WordCleaner,
    '.doc': WordCleaner,
    '.xlsx': ExcelCleaner,
    '.xls': ExcelCleaner,
    '.pptx': PPTCleaner,
    '.ppt': PPTCleaner,
}


class MetadataCleaner:
    """Unified metadata cleaner for all supported file types."""

    @staticmethod
    def is_supported(file_path: str) -> bool:
        """Check if file type is supported."""
        _, ext = os.path.splitext(file_path)
        return ext.lower() in CLEANER_MAP

    @staticmethod
    def get_supported_extensions() -> List[str]:
        """Get list of supported file extensions."""
        return list(CLEANER_MAP.keys())

    @staticmethod
    def get_file_type(file_path: str) -> str:
        """Get human-readable file type name."""
        _, ext = os.path.splitext(file_path)
        ext = ext.lower()

        type_map = {
            '.jpg': 'JPEG Image',
            '.jpeg': 'JPEG Image',
            '.png': 'PNG Image',
            '.gif': 'GIF Image',
            '.webp': 'WebP Image',
            '.tiff': 'TIFF Image',
            '.tif': 'TIFF Image',
            '.bmp': 'BMP Image',
            '.pdf': 'PDF Document',
            '.docx': 'Word Document',
            '.doc': 'Word Document (Legacy)',
            '.xlsx': 'Excel Spreadsheet',
            '.xls': 'Excel Spreadsheet (Legacy)',
            '.pptx': 'PowerPoint Presentation',
            '.ppt': 'PowerPoint Presentation (Legacy)',
        }
        return type_map.get(ext, 'Unknown')

    @staticmethod
    def read_metadata(file_path: str) -> Dict:
        """Read metadata from file."""
        if not MetadataCleaner.is_supported(file_path):
            raise ValueError(f"Unsupported file type: {file_path}")

        _, ext = os.path.splitext(file_path)
        cleaner_class = CLEANER_MAP.get(ext.lower())

        if cleaner_class:
            return cleaner_class.read_metadata(file_path)
        return {}

    @staticmethod
    def clean(input_path: str, output_path: Optional[str] = None) -> str:
        """
        Clean metadata from file.
        Returns the path to the cleaned file.
        """
        if not MetadataCleaner.is_supported(input_path):
            raise ValueError(f"Unsupported file type: {input_path}")

        _, ext = os.path.splitext(input_path)
        cleaner_class = CLEANER_MAP.get(ext.lower())

        if cleaner_class:
            return cleaner_class.clean(input_path, output_path)
        raise ValueError(f"No cleaner available for: {input_path}")
