kind: manual
description: Intel CPU microcode, loaded by the kernel at boot.

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst

variables:
  strip-binaries: ''

config:
  install-commands:
  - |
    install -Dm644 -t '%{install-root}%{prefix}/lib/firmware/intel-ucode' intel-ucode/*
    install -Dm644 license '%{install-root}%{prefix}/share/licenses/intel-microcode/license'

sources:
- kind: tar
  url: github:intel/Intel-Linux-Processor-Microcode-Data-Files/archive/refs/tags/microcode-20260925.tar.gz
  ref: 0ec19023dad1e6dd0c5909174b8dff2555b91cf01671e36271a1ef36655c0cfc
