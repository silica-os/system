kind: meson
description: Thunderbolt device authorization daemon.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/polkit-gobject.bst
- freedesktop-sdk.bst:components/systemd-libs.bst

variables:
  meson-local: >-
    -Dman=false

sources:
- kind: tar
  url: freedesktop:bolt/bolt/-/archive/0.9.11/bolt-0.9.11.tar.gz
  ref: cde4fc6d4a188e37d088cf59a5b2d174ac4b03d57bf9d084d711ba92105c33c0
