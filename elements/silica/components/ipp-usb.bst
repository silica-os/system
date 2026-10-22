kind: manual
description: ipp-usb, driverless IPP over USB printing and scanning (BSD-2-Clause).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-go.bst
- freedesktop-sdk.bst:components/pkg-config.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:components/libusb.bst
- freedesktop-sdk.bst:components/avahi-libs.bst

variables:
  optimize-debug: "false"
  go_buildmode_flag: ""
  local_goflags: "-buildvcs=false"

config:
  build-commands:
  # The release tarball vendors its Go modules (vendor/), so no go_module
  # sources are needed.
  - |
    go build -mod=vendor -tags nethttpomithttp2

  install-commands:
  - |
    install -Dm755 -t "%{install-root}%{sbindir}" ipp-usb
    install -Dm644 -t "%{install-root}%{indep-libdir}/udev/rules.d" systemd-udev/71-ipp-usb.rules
    install -Dm644 -t "%{install-root}%{indep-libdir}/systemd/system" systemd-udev/ipp-usb.service
    install -Dm644 -t "%{install-root}%{sysconfdir}/ipp-usb" ipp-usb.conf
    install -Dm644 -t "%{install-root}%{datadir}/ipp-usb/quirks" ipp-usb-quirks/*
    install -Dm644 -t "%{install-root}%{mandir}/man8" ipp-usb.8

  - |
    %{install-extra}

sources:
- kind: tar
  url: github:OpenPrinting/ipp-usb/archive/refs/tags/0.9.34.tar.gz
  ref: 4f0ada8593fda3f4239a50fcf24a625842143039de97708281e7b8c0cdff201a
