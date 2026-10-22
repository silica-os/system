kind: autotools
description: |
  erofs-utils with SELinux, so mkfs.erofs labels the image from the
  policy's file contexts. A build tool of silica/image/usr.bst only.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:bootstrap/libselinux.bst
- freedesktop-sdk.bst:bootstrap/xxhash.bst
- freedesktop-sdk.bst:bootstrap/xz.bst
- freedesktop-sdk.bst:components/lz4.bst
- freedesktop-sdk.bst:components/util-linux-libs.bst

variables:
  conf-local: >-
    --enable-lzma
    --enable-lz4
    --enable-multithreading
    --with-selinux

sources:
- kind: git_repo
  url: kernel:linux/kernel/git/xiang/erofs-utils.git
  track: v*.*
  ref: v1.9.4-0-gf36cadb5c563995ab3aa8572a60ed6b721b9557d
