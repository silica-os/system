kind: autotools
description: OpenVPN community edition.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst
- freedesktop-sdk.bst:components/systemd.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/iproute2.bst
- freedesktop-sdk.bst:components/libcap-ng.bst
- freedesktop-sdk.bst:components/linux-pam.bst
- freedesktop-sdk.bst:components/lz4.bst
- freedesktop-sdk.bst:components/lzo.bst
- freedesktop-sdk.bst:components/openssl.bst
- freedesktop-sdk.bst:components/systemd-libs.bst

variables:
  conf-local: >-
    --enable-systemd
    --enable-iproute2
    --disable-dco
    --disable-unit-tests

environment:
  IPROUTE: '%{bindir}/ip'
  SYSTEMD_UNIT_DIR: '%{indep-libdir}/systemd/system'
  TMPFILES_DIR: '%{indep-libdir}/tmpfiles.d'

sources:
- kind: tar
  url: openvpn:openvpn-2.7.8.tar.gz
  ref: c070d1d2440b5a6fca6c2c68645c98cd492116ac36ef4f0946177115532c8e36
