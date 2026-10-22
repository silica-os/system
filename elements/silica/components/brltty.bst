kind: autotools
description: Braille display driver and screen reader for the Linux console.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst
- freedesktop-sdk.bst:components/systemd.bst
- silica/components/tcl.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:bootstrap/ncurses.bst
- freedesktop-sdk.bst:components/bluez-libs.bst
- freedesktop-sdk.bst:components/expat.bst
- freedesktop-sdk.bst:components/icu.bst
- freedesktop-sdk.bst:components/libusb.bst
- freedesktop-sdk.bst:components/polkit-gobject.bst

variables:
  # The tarball ships its own build system; it does not support out-of-tree builds.
  build-dir: ''
  conf-local: >-
    --disable-x
    --disable-gpm
    --disable-speech-support
    --disable-python-bindings
    --disable-java-bindings
    --disable-lisp-bindings
    --disable-lua-bindings
    --disable-ocaml-bindings
    --disable-tcl-bindings
    --disable-emacs-bindings
    --with-writable-directory=/run/brltty
    --with-updatable-directory=/var/lib/brltty
  make-install: >-
    make -j1 INSTALL_ROOT="%{install-root}"
    UDEV_PARENT_LOCATION="%{indep-libdir}" install install-systemd install-udev install-polkit

sources:
- kind: tar
  url: brltty:brltty-6.9.1.tar.xz
  ref: cf4619ce1471690e4ecc5544ee6289cef6c5bdf5855a1ac8dcc63f5beedf108d
