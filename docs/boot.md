# Boot

See `base-system.md` §1 for the design (no bootloader, signed UKIs, A/B updates). This file
describes what the tools in `src/` do.

## Boot entries

Silica has no bootloader. The firmware starts a signed UKI from the ESP through its own UEFI
`Boot####` entries. The ESP is mounted at `/boot` on the running system (systemd-gpt-auto picks
`/boot` when it is free; the tools fall back to `/efi`).

### Files on the ESP

- `\EFI\Linux\silica_<version>.efi`: one UKI per installed version (for example
  `silica_26.8.1.efi`).
- `\EFI\BOOT\BOOTX64.EFI`: a copy of the blessed UKI for firmware without (or with lost) NVRAM
  entries. The image ships it; `silica-boot bless` and `silica-install` keep it equal to the
  version that runs (written to a new file, then renamed over the old one).

### Entries

| Description | Points at | Optional data |
| --- | --- | --- |
| `Silica <version>` | `\EFI\Linux\silica_<version>.efi` | none |
| `Silica Recovery` | the UKI of the version that is running (or the newest) | `@1` as UTF-16LE |

- One `Silica <version>` entry per UKI on the ESP, so there is no fixed "A" or "B": the two
  newest versions are the two slots, and older ones are removed when their file is.
- `Silica Recovery` boots the same UKI with the load options string `@1`. systemd-stub reads it
  as "select UKI profile 1", which is the recovery profile.
- An entry is an `EFI_LOAD_OPTION`: `LOAD_OPTION_ACTIVE`, the description in UTF-16LE, a device
  path (Hard Drive node of the ESP with its GPT partition GUID, File Path node, End node), and the
  optional data. The ESP is found from its mount point through `/proc/self/mountinfo` and
  `/sys/dev/block/<major>:<minor>`; its GUID is read from the primary GPT of the parent disk
  (honouring the disk's logical block size).
- Variables are written through efivarfs (`/sys/firmware/efi/efivars/<Name>-<GUID>`), clearing
  the immutable flag first. Attributes are non-volatile, boot service and runtime access.

### Commands

```
silica-boot status
silica-boot add --label L --path '\EFI\Linux\x.efi' [--options @1] [--esp DIR | --esp-device /dev/vda1]
silica-boot next NNNN
silica-boot bless
silica-boot sync [--esp DIR] [--booted VERSION]
```

`add` reuses an entry with the same description, file and data (and repoints it at the given ESP)
and otherwise takes the lowest free `Boot####` number and appends it to `BootOrder`.

### Update, bless, rollback

1. `systemd-sysupdate` writes the new `usr` partitions and the new UKI to the ESP.
2. `silica-boot sync` runs:
   - entries of Silica whose UKI file is gone are deleted (and removed from `BootOrder` and
     `BootNext`); entries that point at another disk's ESP are left alone;
   - `Silica Recovery` is pointed at the UKI that is running (or the newest if that is unknown);
   - a `Silica <version>` entry is created for every UKI without one, placed before
     `Silica Recovery` in `BootOrder` and after the entries already there;
   - if a version newer than the running one exists, `BootNext` is set to it (one boot only).
   The running version is, in order: `--booted`, the Silica entry named by `BootCurrent`, then
   `IMAGE_VERSION` from `os-release`, else the newest UKI (and then nothing is "newer").
   Versions are compared as dotted numbers (`1.10` is newer than `1.9`; `1.0-rc1` is older
   than `1.0`).
3. The machine reboots into the new version. The firmware has cleared `BootNext`.
4. When `boot-complete.target` is reached, `silica-boot-bless.service` runs `silica-boot sync`
   (which also creates the entries on the very first boot, when the firmware came through
   `BOOTX64.EFI`) and then `silica-boot bless`. `bless` moves the running version's entry to the
   front of `BootOrder`: the entry named by `BootCurrent`, or, after a boot through the fallback
   path, the `Silica <IMAGE_VERSION>` entry. It then refreshes `BOOTX64.EFI`. It does nothing in
   Recovery and is idempotent.
5. If the new version never reaches step 4, the next reset uses the old `BootOrder`, which still
   starts the previous version. Every version handed to `BootNext` is recorded in
   `/var/lib/silica/boot/tried`; `sync` does not hand it to `BootNext` again ("did not start last
   time"), and the next newer version is tried normally.

Rollback by hand: the recovery menu (`silica-recovery`, option 2) sets `BootNext` to the Silica
version below the current one (or the newest other one when running the oldest) and reboots.
Blessing then makes it permanent if it boots.

### Installer and recovery

- `silica-install`:
  - lists whole disks from `/sys/block`, skipping loop, zram, sr, ram, dm, md and the disks of the
    running ESP, `/usr` and `/`;
  - asks the user to type the disk name; `--yes --disk /dev/X` skips the questions;
  - runs `systemd-repart --dry-run=no --empty=force --definitions=/usr/lib/silica/install.repart.d
    <disk>`, which copies the ESP and, block by block, the running system partitions;
  - sets the copied `usr` and `usr-verity` partition UUIDs to the halves of the running
    `usrhash=` (repart gives copies new UUIDs, and the UKI finds `/usr` by these);
  - mounts the new ESP under `/run` and keeps only the running version's UKI there, which it also
    makes the fallback loader;
  - runs the same reconciliation as `sync` and puts that version first in `BootOrder`.

  The new disk creates its B slot and data partition on its own first boot, sealed to its own
  TPM.
- `silica-recovery` offers: start Silica, roll back, install, erase all data and reset
  (`systemd-factory-reset request`, else the `FactoryResetRequest` EFI variable), console, shut down.
