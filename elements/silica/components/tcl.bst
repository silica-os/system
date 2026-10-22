kind: autotools
description: |
  Tcl interpreter. Build-time only: brltty generates its udev rules with tclsh.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/zlib.bst

variables:
  command-subdir: unix
  conf-local: >-
    --enable-threads
    --enable-64bit

config:
  install-commands:
    (>):
    - rm -rf '%{install-root}%{datadir}/man'
    - ln -sf tclsh8.6 '%{install-root}%{bindir}/tclsh'

sources:
- kind: tar
  url: sourceforge:tcl/tcl8.6.16-src.tar.gz
  ref: 91cb8fa61771c63c262efb553059b7c7ad6757afa5857af6265e4b0bdc2a14a5
