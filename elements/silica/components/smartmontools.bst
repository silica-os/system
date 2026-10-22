kind: autotools
description: S.M.A.R.T. disk monitoring (smartctl, smartd).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/libcap-ng.bst
- freedesktop-sdk.bst:components/systemd-libs.bst

variables:
  conf-local: >-
    --with-libcap-ng=yes
    --with-libsystemd=yes
    --with-selinux=no
    --with-systemdsystemunitdir=%{indep-libdir}/systemd/system
    --with-initscriptdir=no

sources:
- kind: tar
  url: github:smartmontools/smartmontools/releases/download/RELEASE_7_5/smartmontools-7.5.tar.gz
  ref: 690b83ca331378da9ea0d9d61008c4b22dde391387b9bbad7f29387f2595f76e
