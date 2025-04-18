import os

# File signature definitions (magic numbers) for supported file types
SIGNATURES = {
    "jpg": [b"\xFF\xD8\xFF\xE0", b"\xFF\xD8\xFF\xE1", b"\xFF\xD8\xFF\xDB"],
    "png": [b"\x89\x50\x4E\x47\x0D\x0A\x1A\x0A"],
    "bmp": [b"\x42\x4D"],
    "gif": [b"\x47\x49\x46\x38"],
    "mpo": [b"\xFF\xD8\xFF\xE2"],
    "mp3": [b"\x49\x44\x33"],
    "aac": [b"\xFF\xF1"],
    "wav": [b"\x52\x49\x46\x46"],
    "at3": [b"\x52\x49\x46\x46"],
    "mp4": [b"\x00\x00\x18\x66\x74\x79\x70"],
    "avi": [b"\x52\x49\x46\x46"],
    "m2ts": [b"\x47\x40"],
    "sfo": [b"\x00\x00\x50\x53\x46\x4F"],
    "pkg": [b"\x7F\x50\x4B\x03\x04"],
    "p3t": [b"\x00\x00\x50\x33\x54"],
    "edat": [b"\x00\x00\x00\x00\x00\x01"],
    "self": [b"\x53\x45\x4C\x46\x00\x01"],
    "sprx": [b"\x7F\x45\x4C\x46\x01\x01"],
    "raf": [b"\x52\x41\x46\x00"],
    "rco": [b"\x52\x43\x4F\x00"],
    "dat": [b"\x00\x00\x00\x00"],
    "xml": [b"\x3C\x3F\x78\x6D\x6C\x20"],
    "zip": [b"\x50\x4B\x03\x04"],
    "tar": [b"\x75\x73\x74\x61\x72\x00\x30"],
    "gz": [b"\x1F\x8B\x08"]
}

def extract_files(input_file, output_dir, block_size=512):
    """
    Scans the input disk image file for known file signatures and
    extracts matching data blocks into separate files.

    Args:
        input_file (str): Path to the disk image to analyze.
        output_dir (str): Directory where recovered files will be saved.
        block_size (int): Size of each block to scan (default: 512 bytes).
    """
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    file_count = 0

    with open(input_file, "rb") as f:
        offset = 0
        while True:
            block = f.read(block_size)
            if not block:
                break
            for ext, signatures in SIGNATURES.items():
                for sig in signatures:
                    if block.startswith(sig):
                        filename = os.path.join(output_dir, f"{ext}_{file_count}.{ext}")
                        with open(filename, "wb") as out:
                            out.write(block)
                        print(f"Extracted: {filename}")
                        file_count += 1
                        break  # avoid duplicate matching in the same block
            offset += block_size

    print(f"Extraction complete. {file_count} files recovered.")

if __name__ == "__main__":
    # Customize these paths before running
    input_file = "path/to/decrypted_image.img"
    output_dir = "path/to/output_directory"

    extract_files(input_file, output_dir)
