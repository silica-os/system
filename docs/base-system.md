# Silica base system: what goes in the image

Draft, 2026-10-22. Scope: everything below the desktop shell. The shell, Vetro and the apps come
later and sit on top of this. Source: silica-os#6, revised with the user's decisions of 2026-10-22.

## Principles

- **Complete without the shell.** The base image boots, brings up every device, joins Wi-Fi,
  plays sound, manages users, updates itself and recovers from a bad update with no graphical
  shell installed. A text console is the fallback UI.
- **Hardware works out of the box.** Every mainline driver for PC hardware ships, plus the
  firmware it needs. No "install drivers" step.
- **Built from source by us**, with BuildStream 2 on freedesktop-sdk recipes. Firmware blobs and
  CPU microcode are the only binary inputs.
- **Sealed.** `/usr` is an erofs image under dm-verity, signed by Silica; `/System` is a view into
  it.
- **No bootloader.** The firmware starts the kernel directly.
- **One app format, ours.** No Flatpak, no RPM, no package manager in the image.

## 1. Boot and installation

### Direct boot

- The kernel ships as a **UKI**: one signed EFI executable holding the kernel, initrd, command
  line, os-release and boot splash, wrapped by `systemd-stub`. The stub is part of the kernel image
  and shows no menu, so it is not a bootloader. It measures the UKI into the TPM (PCR 11) and gives
  the kernel its initrd.
- The firmware boots it directly from UEFI boot entries written at install:
  - `Silica <version>`, one per installed UKI (at most two, the A/B slots);
  - `Silica Recovery`: the running UKI with load options `@1`, which selects the UKI's recovery
    profile (§1.4).
  - Details in `docs/boot.md`.
  - A copy of the current UKI also goes to `\EFI\BOOT\BOOTX64.EFI`, for firmware that loses its
    NVRAM entries.
- No GRUB, no systemd-boot, no menu. Choosing a slot by hand is a firmware boot-menu job, like
  holding a key at power-on.

### A/B updates without a bootloader

1. `systemd-sysupdate` writes the new `usr` image, its verity data and its signature to the
   inactive slot, and the new UKI to the ESP.
2. Our update service sets UEFI `BootNext` to the new slot. `BootNext` lasts for one boot only.
3. The new system reaches `boot-complete.target` (health checks pass), and a blessing service
   moves the new slot to the front of `BootOrder`.
4. If the new slot hangs or fails, the next reset falls back to the old `BootOrder`. A hardware
   watchdog makes sure a hang leads to that reset.

### Secure Boot

- Our keys (PK, KEK, db) are enrolled by the installer when the firmware is in Setup Mode. The UKI
  is signed with our db key.
- **Decided (2026-10-22): start with Secure Boot off.**
  - On a PC with only Microsoft's keys a UKI cannot boot with Secure Boot on: Microsoft signs only
    the reviewed `shim` loader for Linux, never a kernel. For now, Silica asks the user to disable
    Secure Boot.
  - The UKI and the images are signed from day one anyway.
  - With Secure Boot off, these still hold:
    - dm-verity with its signed root hash;
    - kernel lockdown, which is forced by our config and does not depend on Secure Boot;
    - TPM-measured boot. The LUKS key is sealed to the signed UKI (PCR 11), so a swapped kernel
      cannot unlock the disk.
  - What is lost: the firmware no longer refuses a foreign kernel before it runs.
  - Optional now: enrol our keys in Setup Mode at install (together with Microsoft's, so GPU option
    ROMs and dual boot keep working).
  - **Decided: start shim-review as soon as possible.** An invisible `shim` (no menu) signed by
    Microsoft will boot our UKI on stock PCs with Secure Boot on. Prerequisites:
    - a legal entity, an EV code-signing certificate and a Microsoft Hardware Dev Center account
      (the user's part);
    - a reproducible shim build with our vendor certificate, SBAT and NX compatibility;
    - a lockdown-enforcing signed kernel;
    - a public security contact and key-handling description.

### Installation

- The installation USB boots the same image, live.
- The installer is a small text program, and graphical later in silica-setup. It shows no
  partitioning screens. It:
  1. picks the target disk;
  2. runs `systemd-repart`, which copies the ESP, `usr-A`, its verity and signature partitions,
     and creates an empty `usr-B` and the LUKS2 `root`;
  3. enrolls the TPM2 (signed PCR policy) and the Secure Boot keys;
  4. writes the UEFI boot entries and reboots.
- First run (user, language, network) happens on the installed system.

### Recovery

- A separate signed **recovery UKI** with its own small image, the equivalent of macOS Recovery.
  From a text menu it offers:
  - reinstall the system;
  - roll back to the other slot;
  - unlock and check disks;
  - a network-capable console for support.

## 2. Kernel

### Source and build

- Mainline **stable** (not LTS), following upstream within a release, from the freedesktop-sdk
  kernel element plus a Silica config fragment.
- Built with Clang and ThinLTO for kCFI.
- Modules are signed at build time with a per-build key, which is then discarded.
  `MODULE_SIG_FORCE` is set.

### Built-in

Only what is needed to reach and mount the root: NVMe, AHCI, virtio-blk/scsi, dm-verity, dm-crypt,
erofs, the root filesystem, vfat, efivarfs, TPM (`tpm_crb`, `tpm_tis`), simpledrm (early console).
Everything else is a module loaded by udev.

### Drivers (signed modules)

The rule: **every mainline driver for hardware found in PCs and laptops sold in the last ~12
years**, as Fedora's config does. Out: ISA/legacy buses, staging, amateur radio, obsolete
filesystems, embedded-SoC-only drivers on x86_64.

- **GPU:**
  - AMD: `amdgpu`, plus `radeon` for pre-GCN cards;
  - Intel: `i915` and `xe`;
  - NVIDIA: `nouveau`, and `nova` as it lands;
  - virtual machines: `virtio-gpu`, `vmwgfx`, `hyperv_drm`, `bochs`.
- **Wi-Fi:**
  - Intel: `iwlwifi`, `iwlmvm`, `iwlmld`;
  - Qualcomm: `ath9k`, `ath10k`, `ath11k`, `ath12k`;
  - MediaTek: `mt76`, including mt7921 and mt7925;
  - Realtek: `rtw88`, `rtw89`, `rtl8xxxu`;
  - Broadcom: `brcmfmac`;
  - Marvell: `mwifiex`.
- **Ethernet:**
  - Intel: `e1000e`, `igb`, `igc`, `ixgbe`;
  - others: `r8169`, `atlantic`, `tg3`, `bnxt`;
  - USB adapters: `cdc_ncm`, `ax88179`, `r8152`;
  - virtual machines: `virtio-net`, `vmxnet3`, `hv_netvsc`.
- **Bluetooth:** `btusb` with `btintel`, `btrtl`, `btmtk`, `btbcm`, and `hci_uart`.
- **Audio:**
  - HDA (`snd-hda-intel` plus codecs);
  - Intel SOF;
  - AMD ACP and SOF;
  - USB audio, HDMI/DP audio;
  - virtio-snd.
- **Input:**
  - generic HID: `hid-generic`, `hid-multitouch`, `i2c-hid`;
  - touchpads and pens: Synaptics, Elan, Wacom;
  - vendor HID: `hid-logitech-hidpp`, `hid-apple` and peers;
  - game controllers: `xpad`, `hid-playstation`, `hid-nintendo`.
- **Storage and buses:**
  - disks and cards: NVMe, AHCI, `usb-storage`, `uas`, SD/MMC (`sdhci`, `rtsx`);
  - USB: xHCI, Thunderbolt/USB4;
  - optical: `sr` for optical drives.
- **Platform and power:**
  - vendor ACPI/WMI: `thinkpad_acpi`, `asus-wmi`, `dell-*`, `hp-wmi`, `ideapad-laptop`,
    `surface_*`, Framework, `apple-gmux`;
  - CPU scaling: `intel_pstate`, `amd-pstate`;
  - thermal: `intel_rapl`, thermal;
  - backlight and lid: ACPI backlight/lid;
  - Intel LPSS/ISH;
  - hybrid GPU switching (`vga_switcheroo`).
- **Sensors and cameras:**
  - sensors: IIO, `hid-sensor-*` (rotation, ambient light);
  - webcams: `uvcvideo`;
  - Intel IPU6/IPU7 for laptop MIPI cameras (with libcamera);
  - TPM.
- **Virtualization:** guest drivers (virtio, Hyper-V, VMware) and **KVM host** (`kvm_intel`,
  `kvm_amd`), so a future virtualization app can run VMs.
- **Filesystems:**
  - for Silica itself: erofs (system), vfat (ESP), and the root filesystem (open point §5);
  - for external disks: exFAT, `ntfs3`, ext4, btrfs, xfs, iso9660/udf, HFS+ read-only;
  - FUSE (MTP, APFS read via userspace later);
  - network: NFS, SMB (`cifs`).

### Security configuration

- **LSMs:** `lockdown=integrity`, IPE, Landlock, BPF-LSM, SELinux. Yama ptrace scope 1.
- **Integrity:**
  - dm-verity with root-hash signature checked against the platform keyring;
  - fs-verity with built-in signatures;
  - Silica keys built in (system, Developer CA);
  - `KEXEC_SIG`.
- **Hardening:**
  - kCFI;
  - `INIT_STACK_ALL_ZERO`, `init_on_alloc=1`, `init_on_free=1`;
  - `RANDOMIZE_KSTACK_OFFSET`, `slab_nomerge`;
  - `HARDENED_USERCOPY`, `FORTIFY_SOURCE`.
- **Removed interfaces:**
  - no `/dev/mem` or `/dev/kmem`, no vsyscall, no a.out, no `uselib`;
  - no unprivileged BPF;
  - unprivileged user namespaces stay on, because the app sandbox needs them.

### Firmware and microcode

- `linux-firmware` (compressed with zstd), `intel-ucode`, `amd-ucode`, SOF firmware and
  `wireless-regdb`, all inside the verity image.
- They are signed binaries, so they are the declared exception to "from source".
- `fwupd` updates device firmware (UEFI capsules, docks, SSDs) from LVFS.

### NVIDIA

- **Default:** `nouveau` with GSP firmware and Mesa **NVK** (Vulkan), plus `zink` for OpenGL. It is
  fully open, built by us, and covers Turing (RTX 20, GTX 16) and newer.
- **Optional:** NVIDIA's open kernel modules (built and signed by us) with NVIDIA's closed
  userspace, as a signed system extension. This is the only closed userspace in the system, so it
  is opt-in.

## 3. Base userspace

Grouped by job. Everything is from freedesktop-sdk recipes unless marked **ours**.

| Job | Components |
|---|---|
| Libraries | glibc, libstdc++, zlib, zstd, xz, OpenSSL, libxcrypt, ICU, libffi, expat, libxml2, pcre2 |
| Init and system | systemd (journald, logind, homed, repart, sysupdate, sysext, measure, pcrlock, resolved, timesyncd, oomd, networkd-wait), udev, kmod, dbus-broker, polkit, Linux-PAM |
| Storage | util-linux, e2fsprogs, btrfs-progs, dosfstools, exfatprogs, cryptsetup, tpm2-tss, smartmontools, nvme-cli |
| Network | NetworkManager with **iwd** for Wi-Fi, ModemManager (cellular), WireGuard (kernel) and OpenVPN, Avahi (mDNS/DNS-SD), ca-certificates, p11-kit |
| Bluetooth | BlueZ |
| Audio and video | PipeWire, WirePlumber, alsa-lib, alsa-ucm-conf, libcamera, FFmpeg (open codecs: AV1/dav1d, VP8/VP9, Opus, FLAC; H.264/HEVC via VA-API hardware and openh264, open point §5) |
| Graphics | Mesa (radeonsi, RADV, iris, ANV, NVK, nouveau, zink, llvmpipe, virgl/Venus), libdrm, Vulkan loader, libva, Wayland libraries, libinput, libxkbcommon, xkeyboard-config |
| Text and fonts | fontconfig, FreeType, HarfBuzz, Silica's fonts, Noto (script coverage), an emoji font |
| Devices | UPower, power-profiles-daemon, thermald (Intel), bolt (Thunderbolt authorization), iio-sensor-proxy, switcheroo-control, fprintd and libfprint, fwupd, geoclue (location) |
| Printing and scanning | CUPS with IPP Everywhere (driverless), cups-filters, ipp-usb, SANE with sane-airscan |
| Accessibility (base) | brltty (braille consoles), espeak-ng and speech-dispatcher. The screen reader comes with the shell. |
| Locale | glibc locales for supported languages, tzdata, hunspell dictionaries |
| Security | SELinux userspace and **our** policy, IPE policy (**ours**), audit |
| Console | Linux VT with fbcon on simpledrm/DRM, agetty and login via PAM and homed, zsh, coreutils, findutils, grep, sed, less, an editor (nano), `run0` instead of sudo |
| Silica services (**ours**) | update service (sysupdate + BootNext blessing), installer, recovery menu, text admin tool `silica` (network, users, updates, disks), Trust1 (bundle verification, fs-verity signing), app launcher and sandbox |

Kept out of the image:

- package managers, Flatpak, ostree, rpm;
- GRUB and systemd-boot;
- Python and Perl;
- compilers (they live in the developer container);
- X11, sudo, debug tools.

Xwayland: open point, §5.

## 4. What the base gives the app format

The app format is its own spec (drag-and-drop install, per-app widgets). From the base it needs:

- **Kernel:**
  - user and mount namespaces;
  - Landlock and seccomp;
  - fs-verity signatures and IPE, so only signed bundle code runs;
  - per-app UIDs, SELinux MCS categories.
- **Services:** Trust1 verifies and seals a bundle when it is copied into `/Applications`. The
  launcher builds the sandbox from the bundle's signed entitlements.
- **Widgets:** a widget is a sandboxed extension inside the bundle, with no network unless entitled.
  The shell hosts and draws it; the base only provides the sandbox and a timeline/refresh scheduler
  (systemd timers per extension).

Third-party Linux apps no longer run as Flatpaks. They must be packaged as Silica bundles: an SDK
and a bundling tool will make that easy for open-source apps.

## 5. Decisions (2026-10-22)

1. **Secure Boot:** off or our own keys at first; shim-review as soon as possible (§1).
2. **Data partition:** btrfs inside LUKS2, unlocked by the TPM2. A TPM 2.0 is required.
3. **Codecs:** open codecs in software; H.264/HEVC only through VA-API hardware decoding and
   openh264.
4. **Xwayland:** not in the base image; later as an optional signed system extension.
5. **Architecture:** x86_64 first. 32-bit compatibility (`IA32_EMULATION`) is off, as on macOS.
6. **Kernel:** mainline stable, following freedesktop-sdk's release (7.2 today).
7. **Flatpak:** dropped. One Silica app format (drag-and-drop install, per-app widgets), its own
   spec.

## 6. Status (2026-10-22)

Built and verified in QEMU (OVMF, swtpm, KVM):

- **Boot:**
  - The firmware starts the signed UKI directly, first through `\EFI\BOOT\BOOTX64.EFI` and
    then through the `Silica <version>` entry that `silica-boot` creates and blesses.
  - The kernel loads Silica's certificate. `/usr` mounts from erofs on dm-verity, and the root
    hash signature is checked by the kernel.
- **First boot:**
  - systemd-repart adds the B slot and the data partition: btrfs in LUKS2, enrolled in the TPM2.
  - `/etc` is filled from the factory defaults.
  - firstboot asks for locale, keymap and timezone (no root password).
  - homed's wizard creates the first user: zsh, `wheel`, a LUKS2 home under `/Users`.
- **Login and administration:** console login works, and `run0` authenticates the `wheel` user
  through polkit.
- **Network:** NetworkManager reaches full connectivity, with systemd-resolved as DNS.
- **No failed units** on a normal boot.
- **Updates:**
  1. `silica-update.service` downloads 0.1.1 into the B slot and the ESP.
  2. `BootNext` points at 0.1.1, and the reboot comes up on 0.1.1 with data and homes intact. The
     TPM unlock survives the update.
  3. 0.1.1 is blessed to the front of BootOrder.
- **Recovery:**
  - The `Silica Recovery` entry (`@1`) shows the menu with the data partition unlocked.
  - "Roll back to the previous version" boots 0.1.0.
- **Installer:**
  - `silica-install --yes --disk /dev/vda` copies the running system to an empty disk in seconds
    and fixes the copied partitions' verity UUIDs.
  - The installed disk boots on its own and builds its B slot and data partition on first boot.
- **IPE:** the policy is active and enforcing.
  - A binary copied to `/tmp` and an executable script in a home both fail with `EACCES`.
  - Everything on the sealed image runs.
  - Scripts still run through their interpreter (`zsh script.sh`).

Found while testing:

- **systemd-repart (v261):**
  - Verity signature partitions must be exactly 16 KiB.
  - It asserts (instead of failing cleanly) when an existing partition does not match that size.
- **OVMF and BootOrder:** OVMF reorders BootOrder when QEMU's disks change, following
  `bootindex`. Real firmware keeps it.

## 7. Not done yet

- **kCFI:** it needs a Clang-built kernel; the kernel is built with GCC for now.
- **SELinux:** freedesktop-sdk's systemd is built without SELinux (`-SELINUX`). The Silica policy
  needs a systemd element of our own first. The LSM is built into the kernel.
- **Signed update manifests:** sysupdate has `Verify=false` until the update server signs
  `SHA256SUMS`. The usr image is still checked by the kernel through its verity signature.
- **Signed TPM policy:** the TPM2 unlock is bound to PCR 7 (Secure Boot state). A signed PCR 11
  policy (`ukify --pcr-private-key`) would also bind it to Silica's UKIs.
- **Missing components:**
  - ipp-usb and speech-dispatcher have no recipe yet.
  - zsh is the shell of homed's first-boot user and of `useradd`; `homectl create` needs
    `--shell=/usr/bin/zsh` (systemd's built-in default is bash).
  - Silica's own recipes are in `elements/silica/components/`.
- **Boot splash:** the boot is verbose. Plymouth and `quiet` come with the graphical shell.
- **Production keys:** the hardware-token signing keys, the update server, and shim-review (§1).
