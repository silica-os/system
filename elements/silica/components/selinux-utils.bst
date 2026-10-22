kind: manual
description: |
  SELinux userspace tools: checkpolicy, which builds the policy, and from
  policycoreutils setfiles/restorecon, load_policy, sestatus and secon.
  freedesktop-sdk ships libselinux and libsepol (with getenforce and the
  other libselinux utilities).

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-make.bst
- freedesktop-sdk.bst:components/flex.bst
- freedesktop-sdk.bst:components/bison.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:bootstrap/libselinux.bst
- freedesktop-sdk.bst:bootstrap/libsepol.bst
- freedesktop-sdk.bst:components/audit.bst

variables:
  make-args: >-
    PREFIX=%{prefix}
    BINDIR=%{bindir}
    SBINDIR=%{sbindir}
    LIBDIR=%{libdir}
    SHLIBDIR=%{libdir}
    LIBSELINUX_LDLIBS=-lselinux

config:
  build-commands:
  # checkpolicy links libsepol statically, and freedesktop-sdk drops
  # libsepol.a: build it from the same release.
  - make -C libsepol/src %{make-args} libsepol.a
  - |
    make -C checkpolicy %{make-args} \
      CPPFLAGS="-I${PWD}/libsepol/include" LIBSEPOLA="${PWD}/libsepol/src/libsepol.a"
  - |
    for tool in setfiles load_policy sestatus secon; do
      make -C "policycoreutils/${tool}" %{make-args}
    done

  install-commands:
  - make -C checkpolicy %{make-args} DESTDIR='%{install-root}' install
  - |
    for tool in setfiles load_policy sestatus secon; do
      make -C "policycoreutils/${tool}" %{make-args} DESTDIR='%{install-root}' install
    done

sources:
- kind: git_repo
  url: github:SELinuxProject/selinux.git
  track: '3.11'
  ref: 3.11-0-g2233a23a4d4f1bf29054037babec13f30d038e65
