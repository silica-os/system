kind: autotools
description: SANE scanner backends (USB and network scanners, no sane-frontends).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst
- freedesktop-sdk.bst:components/_private/python3-minimal.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/avahi-libs.bst
- freedesktop-sdk.bst:components/jpeg.bst
- freedesktop-sdk.bst:components/libpng.bst
- freedesktop-sdk.bst:components/libtiff.bst
- freedesktop-sdk.bst:components/libusb.bst
- freedesktop-sdk.bst:components/libxml2.bst
- freedesktop-sdk.bst:components/systemd-libs.bst

variables:
  conf-local: >-
    --disable-locking
    --disable-latex
    --without-snmp
    --without-v4l
    --without-poppler-glib
    --with-usb
    --with-avahi
    --with-systemd

public:
  bst:
    split-rules:
      devel:
        (>):
        - '%{libdir}/libsane.so'

sources:
- kind: tar
  url: gitlab:-/project/429008/uploads/843c156420e211859e974f78f64c3ea3/sane-backends-1.4.0.tar.gz
  ref: f99205c903dfe2fb8990f0c531232c9a00ec9c2c66ac7cb0ce50b4af9f407a72
