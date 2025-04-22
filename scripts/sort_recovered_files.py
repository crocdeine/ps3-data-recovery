import os
import shutil
from pathlib import Path

# Base recovery directory (change this path as needed)
recovery_dir = Path("/path/to/recovery")

# Output directory for organized files
output_dir = recovery_dir / "organized"
output_dir.mkdir(exist_ok=True)

# File extensions grouped by category
file_types = {
    "photos": [".jpg", ".jpeg", ".png", ".bmp", ".gif", ".mpo"],
    "videos": [".mp4", ".avi", ".m2ts"],
    "music": [".mp3", ".aac", ".wav", ".at3"],
    "documents": [".xml", ".sfo", ".txt"],
    "archives": [".zip", ".tar", ".gz"],
    "ps3_saves": [".bin", ".dat", ".rco", ".p3t", ".pkg", ".edat", ".self", ".sprx", ".raf"],
    "unknown": [],
}

# Determine the category of a file based on its extension
def categorize_file(file_path):
    ext = file_path.suffix.lower()
    for category, extensions in file_types.items():
        if ext in extensions:
            return category
    return "unknown"

# Traverse the recovery directory and move files into categorized folders
for root, _, files in os.walk(recovery_dir):
    for file in files:
        file_path = Path(root) / file
        if file_path.is_file() and output_dir not in file_path.parents:
            category = categorize_file(file_path)
            category_path = output_dir / category
            category_path.mkdir(parents=True, exist_ok=True)
            shutil.move(str(file_path), category_path / file)

print("✅ Sorting complete. Files have been moved to the 'organized/' directory.")
