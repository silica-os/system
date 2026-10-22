kind: meson
description: D-Bus service to check the availability of dual-GPU.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/libgudev.bst
- freedesktop-sdk.bst:components/libdrm.bst

variables:
  meson-local: >-
    -Dgtk_doc=false
    -Dtests=false

sources:
- kind: tar
  url: freedesktop:hadess/switcheroo-control/-/archive/3.0/switcheroo-control-3.0.tar.gz
  ref: 7c690b8e01a88493c43f760f6ab317ea7200e87d17cdeb796a6f4ce81f323537
