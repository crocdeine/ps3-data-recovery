import os
import shutil
import subprocess
import mimetypes
import gzip
import zipfile
import tarfile
from pathlib import Path
from PIL import Image

# Base directory containing organized subfolders with recovered files
organized_dir = Path("path/to/organized")

def is_image_valid(file_path):
    """Check if an image file can be successfully opened."""
    try:
        with Image.open(file_path) as img:
            img.verify()
        return True
    except Exception:
        return False

def is_media_valid(file_path):
    """Use ffmpeg to check if a media file (audio/video) is valid."""
    try:
        result = subprocess.run(
            ['ffmpeg', '-v', 'error', '-i', str(file_path), '-f', 'null', '-'],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
        return result.returncode == 0
    except Exception:
        return False

def is_gzip_valid(file_path):
    """Check if a GZ archive is readable."""
    try:
        with gzip.open(file_path, 'rb') as f:
            f.read(1024)
        return True
    except Exception:
        return False

def is_zip_valid(file_path):
    """Check if a ZIP archive is readable and not corrupted."""
    try:
        with zipfile.ZipFile(file_path, 'r') as z:
            return z.testzip() is None
    except Exception:
        return False

def is_tar_valid(file_path):
    """Check if a TAR archive is readable."""
    try:
        with tarfile.open(file_path, 'r') as t:
            t.getmembers()
        return True
    except Exception:
        return False

def is_text_valid(file_path):
    """Try to read a text-based file."""
    try:
        with open(file_path, 'r', errors='ignore') as f:
            f.read(1024)
        return True
    except Exception:
        return False

# Counters
total_files = 0
corrupted_files = 0

# Traverse each categorized folder in the organized directory
for category_dir in organized_dir.iterdir():
    if category_dir.is_dir():
        corrupted_dir = category_dir / "corrupted"
        corrupted_dir.mkdir(exist_ok=True)

        for file in category_dir.glob("*.*"):
            if file.is_file():
                total_files += 1
                ext = file.suffix.lower()
                is_valid = True

                # File type validation by extension
                if ext in [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".mpo"]:
                    is_valid = is_image_valid(file)
                elif ext in [".mp4", ".avi", ".m2ts", ".mp3", ".aac", ".wav", ".at3"]:
                    is_valid = is_media_valid(file)
                elif ext == ".gz":
                    is_valid = is_gzip_valid(file)
                elif ext == ".zip":
                    is_valid = is_zip_valid(file)
                elif ext == ".tar":
                    is_valid = is_tar_valid(file)
                elif ext in [".txt", ".xml", ".sfo"]:
                    is_valid = is_text_valid(file)
                else:
                    # No check available, assume valid
                    is_valid = True

                if not is_valid:
                    corrupted_files += 1
                    shutil.move(str(file), corrupted_dir / file.name)
                    print(f"[INVALID] {file.name}")
                else:
                    print(f"[VALID]   {file.name}")

# Final summary
print(f"\nIntegrity check complete: {corrupted_files} corrupted file(s) found out of {total_files} checked.")
