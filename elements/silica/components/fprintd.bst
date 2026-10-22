kind: meson
description: Fingerprint authentication daemon and PAM module.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- silica/components/libfprint.bst
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/glib.bst
- freedesktop-sdk.bst:components/linux-pam.bst
- freedesktop-sdk.bst:components/polkit.bst
- freedesktop-sdk.bst:components/systemd-libs.bst

variables:
  meson-local: >-
    -Dpam=true
    -Dman=false
    -Dgtk_doc=false
    -Dsystemd_system_unit_dir=%{indep-libdir}/systemd/system
    -Dpam_modules_dir=%{libdir}/security

sources:
- kind: tar
  url: freedesktop:libfprint/fprintd/-/archive/v1.94.5/fprintd-v1.94.5.tar.gz
  ref: a026ef34c31b25975275cc29a5e4eba2b54524769672095a5228098a08acd82c
