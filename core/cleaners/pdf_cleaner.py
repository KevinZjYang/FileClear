"""PDF metadata cleaner using pypdf."""
import os
from pypdf import PdfReader, PdfWriter


class PDFCleaner:
    """Cleaner for PDF files."""

    SUPPORTED_EXTENSIONS = {'.pdf'}

    @staticmethod
    def is_supported(file_path: str) -> bool:
        """Check if file is a PDF."""
        _, ext = os.path.splitext(file_path)
        return ext.lower() in PDFCleaner.SUPPORTED_EXTENSIONS

    @staticmethod
    def read_metadata(file_path: str) -> dict:
        """Read metadata from PDF file."""
        metadata = {}
        try:
            reader = PdfReader(file_path)

            # Basic info
            metadata['pages'] = len(reader.pages)

            # Document info
            if reader.metadata:
                for key, value in reader.metadata.items():
                    # Remove /Prefix from key names
                    clean_key = key.replace('/', '')
                    if value:
                        metadata[clean_key] = str(value)

            # XMP metadata
            try:
                xmp = reader.xmp_metadata
                if xmp:
                    metadata['xmp_present'] = True
            except Exception:
                pass

        except Exception as e:
            metadata['error'] = str(e)

        return metadata

    @staticmethod
    def clean(input_path: str, output_path: str = None) -> str:
        """
        Clean metadata from PDF file.
        Returns the path to the cleaned file.
        """
        if output_path is None:
            base, ext = os.path.splitext(input_path)
            output_path = f"{base}_cleaned{ext}"

        try:
            reader = PdfReader(input_path)
            writer = PdfWriter()

            # Copy all pages
            for page in reader.pages:
                writer.add_page(page)

            # Remove metadata - explicitly set all fields to empty
            writer.add_metadata({
                '/Title': '',
                '/Author': '',
                '/Subject': '',
                '/Keywords': '',
                '/Creator': '',
                '/Producer': '',
                '/CreationDate': '',
                '/ModDate': '',
            })

            # Write without metadata
            with open(output_path, 'wb') as f:
                writer.write(f)

            return output_path
        except Exception as e:
            raise RuntimeError(f"Failed to clean PDF: {str(e)}")
