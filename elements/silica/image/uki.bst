kind: manual
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
- freedesktop-sdk.bst:components/sbsigntools-maybe.bst
- freedesktop-sdk.bst:components/systemd.bst
- freedesktop-sdk.bst:components/systemd-ukify.bst
- freedesktop-sdk.bst:components/tpm2-tss.bst
- freedesktop-sdk.bst:components/util-linux-full.bst
- silica/kernel/modules.bst
- silica/system/config.bst
- filename: silica/image/usr.bst
  config:
    location: /usr-image
- filename: silica/image/keys.bst
  config:
    location: /keys

variables:
  strip-binaries: ''
  # Kernel command line shared by both profiles. /usr comes from the
  # dm-verity image whose root hash is pinned here; the data partition is
  # found and unlocked by systemd-gpt-auto.
  cmdline-base: 'rw mount.usrflags=ro console=tty0 console=ttyS0'

config:
  build-commands:
  - |
    version="$(ls -1 %{indep-libdir}/modules | head -n1)"
    echo "${version}" >kver
    echo '%{libdir}' >/etc/ld.so.conf
    dracut -v --no-hostonly --reproducible --no-machineid --force \
      --kver "${version}" \
      --kernel-image "%{indep-libdir}/modules/${version}/vmlinuz" \
      --filesystems "btrfs erofs vfat" \
      --libdirs "%{libdir} %{indep-libdir}" \
      --add "systemd-repart systemd-veritysetup crypt tpm2-tss" \
      --omit "plymouth" \
      initrd.img "${version}"

  - |
    hash="$(cat /usr-image/roothash)"
    sig="$(cat /usr-image/roothash.p7s.b64)"
    echo "usrhash=${hash} systemd.verity_usr_options=root-hash-signature=base64:${sig} %{cmdline-base}" >cmdline-silica
    echo "$(cat cmdline-silica) systemd.unit=silica-recovery.target" >cmdline-recovery

  - |
    ukify build \
      --profile='TITLE=Silica Recovery
    ID=recovery' \
      --cmdline=@cmdline-recovery \
      --output=recovery.efi

  - |
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

  install-commands:
  - install -Dm644 silica.efi '%{install-root}/silica_%{silica-version}.efi'
