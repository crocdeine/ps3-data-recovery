# Step-by-step guide: recover your files from a formatted PS3 drive

*[Version française](GUIDE.fr.md)*

This guide is for beginners. You do not need to know how to code, but you will type a few commands; each one is given exactly.

Plan for half a day, mostly because copying the drive takes a while.

## Before you start

**Stop using the PS3.** Formatting does not really erase the data, but every new save, install or update can overwrite the files you want back.

You need:

- **The original PS3.** The encryption key is unique to each console. A drive can only be read with the key of the console it came from.
- **A USB to 2.5" SATA adapter** (around $10), to connect the PS3 drive to your computer.
- **A second, empty 2.5" SATA hard drive**, which stays in the PS3 while you work, so the original drive is never touched.
- **A USB stick** formatted as FAT32.
- **Free space on your computer**: at least **twice the size of the PS3 drive** (one encrypted copy and one decrypted copy). An external drive is fine.

Write down your PS3's **model number**, printed on the back or underneath ("CECH-…"). You need it in step 3.

## Step 1: take the drive out of the PS3

1. Turn the PS3 off and unplug it.
2. Remove the hard drive. Where it sits depends on the model: a side panel on "Fat" consoles, a front or side panel on "Slim" and "Super Slim" ones. iFixit's guides show how for each model.
3. Put this drive somewhere safe. **Do not put it back in the PS3** until the recovery is done.

## Step 2: make an exact copy of the drive

You copy the whole drive, byte for byte, into a file called `ps3_image.img`. Everything else is done on this copy, and the original drive is not touched again.

Connect the drive to the computer with the USB-SATA adapter.

> ⚠️ Your computer will say the disk is unreadable and offer to **initialize** or **format** it. **Always refuse** (click "Ignore" or "Cancel"). Accepting would erase the drive.

### On a Mac

1. Open the **Terminal** app (Applications > Utilities).
2. List the disks by typing this command and pressing Enter:
   ```bash
   diskutil list
   ```
   Find the PS3 drive by its size (for example `*500.1 GB`) and note its name, for example `/dev/disk4`. **Double-check: picking the wrong disk can overwrite your data.**
3. Start the copy, replacing `disk4` with the right number. Add an `r` in front (`rdisk4`), which makes the copy much faster:
   ```bash
   sudo dd if=/dev/rdisk4 of=ps3_image.img bs=1m conv=noerror,sync
   ```
   The Mac asks for your password: type it, even though nothing shows, then press Enter.
4. The copy takes one to several hours. Press **Ctrl + T** to see how far along it is. It is done when the prompt comes back.

### On Windows

Windows has no built-in raw copy tool. Use **HDD Raw Copy Tool** (free, from hddguru.com):

1. Run it as administrator and pick the PS3 drive as the **source**. Recognize it by its size.
2. Pick **FILE** as the target and save as a raw **.img** file (not the compressed `.imgc`).
3. Start the copy.

### On Linux

```bash
lsblk                     # find the PS3 drive, for example /dev/sdb
sudo dd if=/dev/sdb of=ps3_image.img bs=1M status=progress conv=noerror,sync
```

When the copy is done, unplug the original drive and put it away.

## Step 3: get the console's key (`eid_root_key`)

The key is stored inside the PS3 itself. To read it, the console must run **custom firmware (CFW)**.

> ⚠️ **Important limits**
> - **Not every PS3 can install CFW.** Only some "Fat" and "Slim" models can. **Super Slim (CECH-4xxx)** and later Slim models cannot. On those consoles there is currently no known way to get the key, and so no way to decrypt the drive.
> - **HEN is not enough.** The "Homebrew Enabler" cannot extract the key, and users report that trying can leave the console unable to boot.
> - Installing CFW carries a **risk of making the console unusable** ("brick") if the instructions are not followed exactly. You do this at your own risk.
> - Do not sign in to the PlayStation Network while CFW is installed.

1. **Put the second (empty) drive in the PS3.** On start-up the console asks you to reinstall the system software from a USB stick, then formats this drive. That is expected, since it is the empty drive. The guide in the next step explains which file to put on the stick. The original drive stays put away.
2. **Check that your console can run CFW** with the **MinVerChk** tool (see the "PS3:MinVerChk" page on the ConsoleMods wiki). It shows "Update data of version ____ or later can be installed on this system". **If the number is 3.56 or lower, the console is compatible.** Otherwise, stop here.
3. **Install CFW** by following a well-known guide step by step, such as "PS3:How to Hack your PS3" on the ConsoleMods wiki. This guide does not cover that step in detail, because it varies by model and version.
4. **Install Rebug Toolbox** (a `.pkg` file) from a USB stick, using **Install Package Files** in the PS3's Game menu.
5. Open **Rebug Toolbox**, go to the **Utilities** tab and choose **Dump eid root key**. The console restarts.
6. The key is saved to `/dev_hdd0/game/RBGTLBOX2/USRDIR/eid_root_key`. Copy this file to the USB stick with a file manager such as **multiMAN**, or over FTP.
7. On the computer, check that the `eid_root_key` file is exactly **48 bytes**.

> 🔒 This key is unique to your console. **Never share it** or post it anywhere.

## Step 4: install ps3recover

Download the program for your computer from the project's [Releases](https://github.com/crocdeine/ps3-data-recovery/releases) page:

| Computer | File |
|---|---|
| Recent Mac (M1, M2… chip) | `ps3recover-macos-apple-silicon.tar.gz` |
| Intel Mac | `ps3recover-macos-intel.tar.gz` |
| Windows | `ps3recover-windows-x86_64.zip` |
| Linux | `ps3recover-linux-x86_64.tar.gz` |

Unpack it and put `ps3recover` (or `ps3recover.exe`) in the same folder as `ps3_image.img` and `eid_root_key`.

Open a terminal in that folder:

- **Mac**: in Terminal, type `cd ` (with a space), drag the folder into the window, and press Enter. macOS blocks programs downloaded outside the App Store; unblock this one once with:
  ```bash
  xattr -d com.apple.quarantine ps3recover
  ```
- **Windows**: open the folder in File Explorer, right-click an empty area and choose "Open in Terminal". In the commands below, replace `./ps3recover` with `.\ps3recover.exe`.

## Step 5: decrypt the drive

```bash
./ps3recover decrypt ps3_image.img ps3_decrypted.img --key eid_root_key
```

- If the key is right, it prints `Key accepted: drive detected as …` and then the progress.
- If the key does not match the drive, it stops straight away with an error and writes nothing. Check that the key comes from the console the drive came from.

## Step 6: get your files back

```bash
./ps3recover recover ps3_decrypted.img recovered_files
```

The program scans the whole decrypted drive and sorts what it finds into the `recovered_files` folder:

| Folder | Contents |
|---|---|
| `photos` | Photos and screenshots (jpg, png, gif…) |
| `videos` | Videos (mp4, PS3 recordings in m2ts…) |
| `music` | Music (mp3, wav…) |
| `ps3_files` | PS3 files: save information (`.sfo`), games and updates (`.pkg`)… |
| `documents`, `archives` | Text files and archives (zip…) |

In each folder, a `corrupted` subfolder holds damaged files, often because they were split into several pieces on the drive. To have videos and music checked too, install [ffmpeg](https://ffmpeg.org/) before running the command.

Files are named with hexadecimal digits, which give their position on the drive. The original names cannot be recovered, because formatting erased them.

## Good to know

- **Nothing is guaranteed.** Anything overwritten since the format is gone.
- **Game saves** are harder to recover than photos and videos: they are made of several files, often encrypted by the game itself. The tool finds their `PARAM.SFO` files, which at least tells you which games they belong to.
- **Something went wrong?** Open an [issue](https://github.com/crocdeine/ps3-data-recovery/issues) with your PS3 model and the message shown, **never attaching your key**.
