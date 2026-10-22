kind: meson
description: SANE backend for driverless scanning (eSCL and WSD).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst

depends:
- silica/components/sane-backends.bst
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/avahi-libs.bst
- freedesktop-sdk.bst:components/gnutls.bst
- freedesktop-sdk.bst:components/jpeg.bst
- freedesktop-sdk.bst:components/libpng.bst
- freedesktop-sdk.bst:components/libtiff.bst
- freedesktop-sdk.bst:components/libxml2.bst

sources:
- kind: tar
  url: github:alexpevzner/sane-airscan/archive/refs/tags/0.99.38.tar.gz
  ref: 1e5b3b8373141caa37bf42fe74ae1950de3ff15c84966d8a415485d334ba5507
