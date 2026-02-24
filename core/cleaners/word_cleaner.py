"""Word document metadata cleaner."""
import os
import shutil
import tempfile
import zipfile
from pathlib import Path


class WordCleaner:
    """Cleaner for Word documents (.docx and .doc)."""

    SUPPORTED_EXTENSIONS = {'.docx', '.doc'}

    @staticmethod
    def is_supported(file_path: str) -> bool:
        """Check if file is a Word document."""
        _, ext = os.path.splitext(file_path)
        return ext.lower() in WordCleaner.SUPPORTED_EXTENSIONS

    @staticmethod
    def read_metadata(file_path: str) -> dict:
        """Read metadata from Word document."""
        metadata = {}
        _, ext = os.path.splitext(file_path)

        if ext.lower() == '.docx':
            return WordCleaner._read_docx_metadata(file_path)
        else:
            return WordCleaner._read_doc_metadata(file_path)

    @staticmethod
    def _read_docx_metadata(file_path: str) -> dict:
        """Read metadata from .docx file."""
        metadata = {}
        try:
            with zipfile.ZipFile(file_path, 'r') as zip_file:
                # Read core.xml (Dublin Core metadata)
                if 'docProps/core.xml' in zip_file.namelist():
                    import xml.etree.ElementTree as ET
                    content = zip_file.read('docProps/core.xml')
                    root = ET.fromstring(content)

                    # Namespace for dc elements
                    ns = {'dc': 'http://purl.org/dc/elements/1.1/',
                          'cp': 'http://schemas.openxmlformats.org/package/2006/metadata/core-properties'}

                    for elem in root.iter():
                        tag = elem.tag
                        if '}' in tag:
                            tag = tag.split('}')[1]
                        if elem.text and elem.text.strip():
                            metadata[tag] = elem.text.strip()

                # Read app.xml (Application metadata)
                if 'docProps/app.xml' in zip_file.namelist():
                    import xml.etree.ElementTree as ET
                    content = zip_file.read('docProps/app.xml')
                    root = ET.fromstring(content)

                    for elem in root.iter():
                        tag = elem.tag
                        if '}' in tag:
                            tag = tag.split('}')[1]
                        if elem.text and elem.text.strip() and tag not in ['Application', 'AppVersion']:
                            metadata[tag] = elem.text.strip()

        except Exception as e:
            metadata['error'] = str(e)

        return metadata

    @staticmethod
    def _read_doc_metadata(file_path: str) -> dict:
        """Read metadata from old .doc file."""
        metadata = {}
        try:
            import olefile
            ole = olefile.OleFileIO(file_path)

            # Read summary information
            if ole.exists('\x05SummaryInformation'):
                summary = ole.getstream('\x05SummaryInformation')
                props = WordCleaner._parse_summary_info(summary)
                metadata.update(props)

            if ole.exists('\x05DocumentSummaryInformation'):
                doc_summary = ole.getstream('\x05DocumentSummaryInformation')
                props = WordCleaner._parse_summary_info(doc_summary)
                metadata.update(props)

            ole.close()
        except Exception as e:
            metadata['error'] = str(e)

        return metadata

    @staticmethod
    def _parse_summary_info(data: bytes) -> dict:
        """Parse SummaryInformation stream."""
        import struct
        props = {}

        try:
            # Skip header (first 32 bytes typically)
            offset = 0
            while offset < len(data) - 4:
                # Read property ID and offset
                prop_id = struct.unpack('<I', data[offset:offset+4])[0]
                offset += 4
                prop_offset = struct.unpack('<I', data[offset:offset+4])[0]
                offset += 4

                if prop_offset > len(data) - 4:
                    break

                # Try to read as UTF-16 string
                try:
                    # Find null terminator
                    end = prop_offset
                    while end < len(data) - 1 and data[end:end+2] != b'\x00\x00':
                        end += 2

                    if end > prop_offset:
                        value = data[prop_offset:end].decode('utf-16le').strip('\x00')
                        if value:
                            prop_names = {
                                0x01: 'Title',
                                0x02: 'Subject',
                                0x03: 'Author',
                                0x04: 'Keywords',
                                0x05: 'Comments',
                                0x08: 'LastModifiedBy',
                                0x0C: 'Created',
                                0x0D: 'Modified',
                            }
                            props[prop_names.get(prop_id, f'Property_{prop_id}')] = value
                except Exception:
                    pass

        except Exception:
            pass

        return props

    @staticmethod
    def clean(input_path: str, output_path: str = None) -> str:
        """
        Clean metadata from Word document.
        Returns the path to the cleaned file.
        """
        _, ext = os.path.splitext(input_path)

        if ext.lower() == '.docx':
            return WordCleaner._clean_docx(input_path, output_path)
        else:
            return WordCleaner._clean_doc(input_path, output_path)

    @staticmethod
    def _clean_docx(input_path: str, output_path: str = None) -> str:
        """Clean metadata from .docx file."""
        if output_path is None:
            base, ext = os.path.splitext(input_path)
            output_path = f"{base}_cleaned{ext}"

        try:
            # Extract to temp directory
            temp_dir = tempfile.mkdtemp()

            try:
                # Extract original
                shutil.unpack_archive(input_path, temp_dir, 'zip')

                # Clear core.xml
                core_xml_path = Path(temp_dir) / 'docProps' / 'core.xml'
                if core_xml_path.exists():
                    WordCleaner._clear_core_xml(core_xml_path)

                # Clear app.xml
                app_xml_path = Path(temp_dir) / 'docProps' / 'app.xml'
                if app_xml_path.exists():
                    WordCleaner._clear_app_xml(app_xml_path)

                # Repack
                shutil.make_archive(output_path.replace('.docx', ''), 'zip', temp_dir)
                shutil.move(output_path.replace('.docx', '') + '.zip', output_path)

            finally:
                shutil.rmtree(temp_dir, ignore_errors=True)

            return output_path
        except Exception as e:
            raise RuntimeError(f"Failed to clean Word document: {str(e)}")

    @staticmethod
    def _clear_core_xml(file_path: Path):
        """Clear core.xml metadata."""
        import xml.etree.ElementTree as ET

        # Read original
        tree = ET.parse(file_path)
        root = tree.getroot()

        # Remove all text content from elements
        for elem in root.iter():
            if elem.text:
                elem.text = ''

        # Write back
        tree.write(file_path, encoding='UTF-8', xml_declaration=True)

    @staticmethod
    def _clear_app_xml(file_path: Path):
        """Clear app.xml metadata but keep essential info."""
        import xml.etree.ElementTree as ET

        # Read original
        tree = ET.parse(file_path)
        root = tree.getroot()

        # Keep only TotalTime, but clear other properties
        for elem in root.iter():
            if elem.tag.endswith('TotalTime'):
                continue
            if elem.text:
                elem.text = ''

        # Write back
        tree.write(file_path, encoding='UTF-8', xml_declaration=True)

    @staticmethod
    def _clean_doc(input_path: str, output_path: str = None) -> str:
        """Clean metadata from old .doc file."""
        # For .doc files, we copy the file as-is for now
        # Full .doc metadata cleaning requires more complex handling
        if output_path is None:
            base, _ = os.path.splitext(input_path)
            output_path = f"{base}_cleaned.doc"

        shutil.copy2(input_path, output_path)

        # Try to clean with olefile
        try:
            import olefile
            ole = olefile.OleFileIO(output_path)

            # Clear SummaryInformation
            if ole.exists('\x05SummaryInformation'):
                # This is a simplified approach - full implementation would rewrite the stream
                pass

            ole.close()
        except Exception:
            pass

        return output_path
