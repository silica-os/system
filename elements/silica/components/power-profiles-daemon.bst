kind: meson
description: Power profiles over D-Bus (power-saver, balanced, performance).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/libgudev.bst
- freedesktop-sdk.bst:components/polkit-gobject.bst
- freedesktop-sdk.bst:components/upower.bst

variables:
  meson-local: >-
    -Dgtk_doc=false
    -Dpylint=disabled
    -Dtests=false
    -Dmanpage=disabled
    -Dbashcomp=disabled
    -Dzshcomp=

sources:
- kind: tar
  url: freedesktop:upower/power-profiles-daemon/-/archive/0.30/power-profiles-daemon-0.30.tar.gz
  ref: 90f2c95024861d9f4062abedd514084939224e7a49edd468f001af7260ec8e6c
