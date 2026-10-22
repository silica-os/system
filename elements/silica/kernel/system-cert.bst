kind: manual
description: |
  Silica's system signing certificate, staged where the kernel build looks
  for its trusted keys (fdsdk-config.sh reads /keys/linux-module-cert.crt).
  It signs kernel modules and the system image's dm-verity root hash.

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst

config:
  install-commands:
  - install -Dm644 system.crt '%{install-root}/keys/linux-module-cert.crt'

sources:
- kind: local
  path: keys/system.crt
