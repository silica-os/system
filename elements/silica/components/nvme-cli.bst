kind: meson
description: NVMe management command line interface.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/json-c.bst
- freedesktop-sdk.bst:components/libnvme.bst
- freedesktop-sdk.bst:components/systemd-libs.bst

variables:
  meson-local: >-
    -Dpython=disabled
    -Ddocs=false
    -Dtests=false
    -Dexamples=false
    -Dsystemddir=lib/systemd/system
    -Dudevrulesdir=lib/udev/rules.d

sources:
- kind: tar
  url: github:linux-nvme/nvme-cli/archive/refs/tags/v3.1.tar.gz
  ref: 8a6879f1f2af53743c03a8f77f247261306300d3f3b0edfd242c5f646bdfdccb
