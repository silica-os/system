kind: meson
description: Fingerprint reader library.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/libgudev.bst
- freedesktop-sdk.bst:components/libgusb.bst
- freedesktop-sdk.bst:components/nss.bst
- freedesktop-sdk.bst:components/pixman.bst

variables:
  meson-local: >-
    -Dintrospection=false
    -Dgtk-examples=false
    -Ddoc=false
    -Dinstalled-tests=false

public:
  bst:
    split-rules:
      devel:
        (>):
        - '%{libdir}/libfprint-2.so'

sources:
- kind: tar
  url: freedesktop:libfprint/libfprint/-/archive/v1.94.10/libfprint-v1.94.10.tar.gz
  ref: 8d5b202e836f69076c90ead0a654b0ee275788922359ebc9d7706bb5cde7ca54
