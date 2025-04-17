sudo ./decrypt/ps3_dump_decrypted -m cbc -o 0 ps3_image.img ps3_decrypted.img eid_root_key
photorec /d recovered_files /cmd ps3_decrypted.img options,search
