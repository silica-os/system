kind: autotools
description: Intel thermal daemon.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst
- freedesktop-sdk.bst:components/systemd.bst
- freedesktop-sdk.bst:components/gtk-doc.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/libevdev.bst
- freedesktop-sdk.bst:components/libxml2.bst
- freedesktop-sdk.bst:components/upower.bst

variables:
  autogen: NOCONFIGURE=1 ./autogen.sh
  conf-local: >-
    --disable-werror
    --with-systemdsystemunitdir=%{indep-libdir}/systemd/system
    --with-dbus-sys-dir=%{datadir}/dbus-1/system.d

sources:
- kind: tar
  url: github:intel/thermal_daemon/archive/refs/tags/v2.15.10.tar.gz
  ref: 91f0eb6a471637cd16877be59947048c90912ea80495f78f8c2d4f465f12d6a0
