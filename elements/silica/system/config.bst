kind: manual
description: Silica's system configuration, layout and services.

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst
- freedesktop-sdk.bst:components/findutils.bst
- freedesktop-sdk.bst:components/sed.bst

variables:
  strip-binaries: ''
  # Development images update from the host of the QEMU VM.
  update-url: 'http://10.0.2.2:8080/'
  # Slot sizes; repart.d, the installer and the disk image must agree.
  usr-size: '6G'
  verity-size: '128M'
  sig-size: '1M'

config:
  install-commands:
  - |
    cp -a usr "%{install-root}/usr"
    cp -a etc "%{install-root}/etc"
    find "%{install-root}/usr" -type f -exec sed -i \
      -e 's|@VERSION@|%{silica-version}|g' \
      -e 's|@UPDATE_URL@|%{update-url}|g' \
      -e 's|@USR_SIZE@|%{usr-size}|g' \
      -e 's|@VERITY_SIZE@|%{verity-size}|g' \
      -e 's|@SIG_SIZE@|%{sig-size}|g' {} +
    ln -s ../lib/os-release "%{install-root}/usr/share/silica/os-release"

  - |
    # /System: the macOS-style view of the sealed system image.
    s="%{install-root}/usr/System"
    mkdir -p "$s/Applications" "$s/Library/Frameworks" "$s/Library/CoreServices" "$s/Library/Extensions"
    ln -s ../../lib/modules "$s/Library/Kernels"
    ln -s ../../lib/firmware "$s/Library/Firmware"
    ln -s ../../share/fonts "$s/Library/Fonts"
    # systemd-sysext looks for system extensions here.
    ln -s ../System/Library/Extensions "%{install-root}/usr/lib/extensions"

sources:
- kind: local
  path: files/system
