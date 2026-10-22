kind: script
description: |
  The unified kernel image the firmware starts directly: kernel, initrd,
  command line and os-release, signed for Secure Boot. Profile 0 boots
  Silica; profile 1 (load options "@1") boots Silica Recovery.

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst
- freedesktop-sdk.bst:components/binutils.bst
- freedesktop-sdk.bst:components/btrfs-progs.bst
- freedesktop-sdk.bst:components/cryptsetup.bst
- freedesktop-sdk.bst:components/dracut.bst
- freedesktop-sdk.bst:components/kmod.bst
- freedesktop-sdk.bst:components/sed.bst
- freedesktop-sdk.bst:components/findutils.bst
- freedesktop-sdk.bst:components/grep.bst
- freedesktop-sdk.bst:components/lvm2.bst
- freedesktop-sdk.bst:components/sbsigntools-maybe.bst
- freedesktop-sdk.bst:components/systemd.bst
- freedesktop-sdk.bst:components/systemd-ukify.bst
- freedesktop-sdk.bst:components/tpm2-tss.bst
- freedesktop-sdk.bst:components/tpm2-tools.bst
- freedesktop-sdk.bst:components/util-linux-full.bst
- freedesktop-sdk.bst:components/zstd.bst
- freedesktop-sdk.bst:components/gzip.bst
- silica/kernel/modules.bst
- silica/system/config.bst
- filename: silica/image/usr.bst
  config:
    location: /usr-image
- filename: silica/image/keys.bst
  config:
    location: /keys

variables:
  # Kernel command line shared by both profiles. /usr comes from the
  # dm-verity image whose root hash is pinned here; the data partition is
  # found and unlocked by systemd-gpt-auto. The last console= is
  # /dev/console, where first-boot questions and Recovery appear: the
  # screen. The serial port still gets kernel messages and a login.
  cmdline-base: 'rw mount.usrflags=ro console=ttyS0 console=tty0'

config:
  commands:
  - mkdir -p /work
  - |
    cd /work
    version="$(ls -1 %{indep-libdir}/modules | head -n1)"
    echo "${version}" >kver
    echo '%{libdir}' >/etc/ld.so.conf
    mkdir -p dracut-tmp
    dracut -v --zstd --tmpdir "${PWD}/dracut-tmp" --no-hostonly --reproducible --no-machineid --force \
      --kver "${version}" \
      --kernel-image "%{indep-libdir}/modules/${version}/vmlinuz" \
      --filesystems "btrfs erofs vfat" \
      --libdirs %{libdir} \
      --libdirs %{indep-libdir} \
      --add "systemd-repart systemd-veritysetup crypt tpm2-tss" \
      --omit "plymouth" \
      initrd.img "${version}"

  - |
    cd /work
    hash="$(cat /usr-image/roothash)"
    sig="$(cat /usr-image/roothash.p7s.b64)"
    echo "usrhash=${hash} systemd.verity_usr_options=root-hash-signature=base64:${sig} %{cmdline-base}" >cmdline-silica
    echo "$(cat cmdline-silica) systemd.unit=silica-recovery.target" >cmdline-recovery

  - |
    cd /work
    ukify build \
      --profile='TITLE=Silica Recovery
    ID=recovery' \
      --cmdline=@cmdline-recovery \
      --output=recovery.efi

  - |
    cd /work
    version="$(cat kver)"
    ukify build \
      --linux="%{indep-libdir}/modules/${version}/vmlinuz" \
      --initrd=initrd.img \
      --uname="${version}" \
      --os-release=@%{indep-libdir}/os-release \
      --cmdline=@cmdline-silica \
      --profile='TITLE=Silica
    ID=silica' \
      --join-profile=recovery.efi \
      --signtool=sbsign \
      --secureboot-private-key=/keys/secureboot-db.key \
      --secureboot-certificate=/keys/secureboot-db.crt \
      --output=silica.efi

  - install -Dm644 /work/silica.efi '%{install-root}/silica_%{silica-version}.efi'
