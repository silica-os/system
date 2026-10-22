# Silica system

The base of Silica OS: a Linux system built from source, sealed and image-based, booted directly by
the firmware with no bootloader. This repository builds everything below the desktop shell, and
the result is a complete system you can use from the console.

- **Design:** `docs/base-system.md` (what goes in the image and why).
- **Boot:** `docs/boot.md` (UEFI entries, A/B updates, recovery).

## What it builds

- **Kernel:** Linux 7.2 with Silica's hardening on top of freedesktop-sdk's configuration
  (`files/linux/silica-config.sh`):
  - module signing is enforced;
  - lockdown runs in integrity mode;
  - IPE and fs-verity signatures;
  - Landlock, Yama, SELinux and BPF-LSM;
  - no legacy interfaces.
- **System image:** `/usr` as erofs on dm-verity. Its root hash is signed with Silica's key and
  checked by the kernel.
- **UKI:** kernel, initrd and command line in one signed EFI binary that the firmware starts
  directly. Profile 0 boots Silica; profile 1 (`@1`) boots Silica Recovery.
- **First boot:** `systemd-repart` adds the B slot and the data partition (btrfs in LUKS2,
  unlocked by the TPM2).
- **Users:** systemd-homed with encrypted homes under `/Users`. The first user is the
  administrator.
- **Updates:**
  1. `systemd-sysupdate` writes the next version into the free slot.
  2. `silica-boot sync` points `BootNext` at it.
  3. After a good boot, `silica-boot bless` makes it the default.
- **Layout:** `/System` (a view into the sealed image), `/Applications`, `/Library` and `/Users`.

## Building

You need Docker and about 30 GB of disk space. BuildStream 2 runs in freedesktop-sdk's `bst2`
container. Elements that Silica does not change come prebuilt from freedesktop-sdk's artifact
cache; everything else is compiled here.

```sh
tools/gen-dev-keys          # development signing keys in keys/ (never committed)
tools/checkout-image        # builds silica/image/disk.bst into out/image/
tools/run-vm --fresh        # QEMU + OVMF + swtpm
```

`out/image/silica.img` is the disk image. `out/image/update/` holds the payload that
`systemd-sysupdate` downloads; development images fetch it from `http://10.0.2.2:8080/`, the QEMU
host.

### Testing in QEMU without a display

- `tools/run-vm --headless --serial-socket /dev/shm/vm.sock --debug-socket /dev/shm/dbg.sock`
  - The debug socket gives a root shell on ttyS1.
  - It works only with Secure Boot off, because systemd-stub accepts the extra kernel argument only
    then.
- `tools/vm-console SOCKET LOG 'expect:login:' 'send:...'` drives either console from scripts.
- `tools/test-vm [--update VERSION] [--install]` boots a fresh image and checks the base system
  (verity, encryption and TPM policy, user, network, IPE, A/B update, installer).

## Layout of the repository

| Path | What |
|---|---|
| `elements/silica/kernel/` | Kernel, signed modules |
| `elements/silica/components/` | Components freedesktop-sdk does not have |
| `elements/silica/system/` | Configuration, tools, the full package set |
| `elements/silica/image/` | Sealed `/usr`, UKI, disk image |
| `files/system/` | Files installed as-is (systemd units, repart, sysupdate, tmpfiles) |
| `src/` | Rust tools: `silica-boot`, `silica-install`, `silica-recovery` |
| `tools/` | Build and VM helpers |

## License

MIT or Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). Elements and files derived
from freedesktop-sdk keep their MIT license headers.
