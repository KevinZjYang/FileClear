"""Image metadata cleaner using Pillow."""
import os
from PIL import Image
from PIL.ExifTags import TAGS


class ImageCleaner:
    """Cleaner for image files (JPG, PNG, GIF, WebP, TIFF, BMP)."""

    SUPPORTED_EXTENSIONS = {'.jpg', '.jpeg', '.png', '.gif', '.webp', '.tiff', '.tif', '.bmp'}

    @staticmethod
    def is_supported(file_path: str) -> bool:
        """Check if file is a supported image format."""
        _, ext = os.path.splitext(file_path)
        return ext.lower() in ImageCleaner.SUPPORTED_EXTENSIONS

    @staticmethod
    def read_metadata(file_path: str) -> dict:
        """Read metadata from image file."""
        metadata = {}
        try:
            with Image.open(file_path) as img:
                # Basic info
                metadata['format'] = img.format
                metadata['mode'] = img.mode
                metadata['size'] = img.size

                # EXIF data
                exif_data = img.getexif()
                if exif_data:
                    exif_info = {}
                    for tag_id, value in exif_data.items():
                        tag = TAGS.get(tag_id, tag_id)
                        # Truncate long values
                        value_str = str(value)
                        if len(value_str) > 200:
                            value_str = value_str[:200] + '...'
                        exif_info[str(tag)] = value_str
                    if exif_info:
                        metadata['exif'] = exif_info

                # ICC profile
                if hasattr(img, 'icc_profile') and img.icc_profile:
                    metadata['icc_profile'] = f"{len(img.icc_profile)} bytes"

                # PNG info chunks
                if img.format == 'PNG':
                    png_info = {}
                    if hasattr(img, 'info'):
                        for key in ['Creation Time', 'Description', 'Author', 'Software']:
                            if key.lower() in [k.lower() for k in img.info.keys()]:
                                for k, v in img.info.items():
                                    if k.lower() == key.lower():
                                        png_info[key] = str(v)
                                        break
                        if png_info:
                            metadata['png_info'] = png_info
        except Exception as e:
            metadata['error'] = str(e)

        return metadata

    @staticmethod
    def clean(input_path: str, output_path: str = None) -> str:
        """
        Clean metadata from image file.
        Returns the path to the cleaned file.
        """
        if output_path is None:
            base, ext = os.path.splitext(input_path)
            output_path = f"{base}_cleaned{ext}"

        try:
            with Image.open(input_path) as img:
                # Get the data (without metadata)
                data = img.getdata()

                # Create a new image without metadata
                # For JPEG, we need to convert to RGB if it's RGBA
                if img.format == 'JPEG' and img.mode in ('RGBA', 'P'):
                    # Convert to RGB to avoid palette issues
                    clean_img = Image.new('RGB', img.size)
                    clean_img.paste(img)
                else:
                    # Create new image with same mode and size
                    clean_img = Image.new(img.mode, img.size)
                    clean_img.putdata(data)

                # Save without metadata
                # For JPEG, use quality=95 to maintain good quality
                save_kwargs = {}
                if img.format == 'JPEG':
                    save_kwargs['quality'] = 95
                    save_kwargs['optimize'] = True
                elif img.format == 'PNG':
                    save_kwargs['optimize'] = True

                clean_img.save(output_path, **save_kwargs)

            return output_path
        except Exception as e:
            raise RuntimeError(f"Failed to clean image: {str(e)}")
