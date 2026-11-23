#!/usr/bin/env python3
"""Regenerate the decryption test fixtures.

The fixtures are encrypted with OpenSSL (through Python's `cryptography`
package), not by ps3recover, so the decryption tests check ps3recover against
an independent implementation. Run from this folder:

    pip install cryptography
    python3 make_fixtures.py
"""
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

# Fake eid_root_key (the "test fake keys" one from ps3hdd_keygen.sh).
ERK = bytes.fromhex(
    "000102030405060708090A0B0C0D0E0F544553540000544553540000544553540F0E0D0C0B0A09080706050403020100"
)
ATA_DATA_SEED = "D92D65DB057D49E1A66F2274B8BAC50883844ED756CA79516362EA8ADAC60326"
ATA_TWEAK_SEED = "C3B3B5AACC74CD6A48EFABF44DCDF16E379F55F5777D09FBEEDE07058E94BE08"
SECTOR = 512


def encrypt(mode, key, data):
    enc = Cipher(algorithms.AES(key), mode).encryptor()
    return enc.update(data) + enc.finalize()


def bswap16(data):
    out = bytearray(data)
    out[0::2], out[1::2] = data[1::2], data[0::2]
    return bytes(out)


data_key = encrypt(modes.CBC(ERK[32:]), ERK[:32], bytes.fromhex(ATA_DATA_SEED))
tweak_key = encrypt(modes.CBC(ERK[32:]), ERK[:32], bytes.fromhex(ATA_TWEAK_SEED))

# Four sectors: a sector 0 carrying the PS3 partition table magic, then a pattern.
sector0 = bytearray(SECTOR)
sector0[0x14:0x18] = bytes.fromhex("0FACE0FF")
sector0[0x1C:0x20] = bytes.fromhex("DEADFACE")
sectors = [bytes(sector0)] + [bytes((i * 7 + s) % 256 for i in range(SECTOR)) for s in range(1, 4)]

slim = b"".join(
    bswap16(encrypt(modes.XTS(n.to_bytes(16, "little")), data_key[:16] + tweak_key[:16], s))
    for n, s in enumerate(sectors)
)
fat = b"".join(bswap16(encrypt(modes.CBC(bytes(16)), data_key[:24], s)) for s in sectors)

open("fake_eid_root_key.bin", "wb").write(ERK)
open("plain.bin", "wb").write(b"".join(sectors))
open("slim.img", "wb").write(slim)
open("fat.img", "wb").write(fat)
print("ata data key: ", data_key.hex())
print("ata tweak key:", tweak_key.hex())
