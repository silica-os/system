kind: manual
description: |
  The kernel and its modules, each module signed with Silica's system key
  (the kernel refuses unsigned ones) and compressed with zstd.

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst
- freedesktop-sdk.bst:components/findutils.bst
- freedesktop-sdk.bst:components/kmod.bst
- freedesktop-sdk.bst:components/sign-file.bst
- freedesktop-sdk.bst:components/zstd.bst
- silica/kernel/linux.bst

variables:
  strip-binaries: ''

config:
  install-commands:
  - |
    mkdir -p "%{install-root}%{indep-libdir}"
    cp -rT %{indep-libdir}/modules "%{install-root}%{indep-libdir}/modules"
    # Build trees and debug data stay out of the image.
    find "%{install-root}%{indep-libdir}/modules" -maxdepth 2 \( -name build -o -name source -o -name vmlinux -o -name System.map \) -exec rm -rf {} +

  - openssl x509 -inform PEM -outform DER -in system.crt -out system.cer

  - |
    find '%{install-root}%{indep-libdir}/modules' -type f -name '*.ko' \
      -exec sign-file sha512 system.key system.cer {} ';' -exec zstd -q --rm -19 {} ';'

  - |
    for version in $(ls "%{install-root}%{indep-libdir}/modules"); do
      depmod -b "%{install-root}%{prefix}" "${version}"
    done

sources:
- kind: local
  path: keys/system.key
- kind: local
  path: keys/system.crt
