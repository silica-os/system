kind: autotools
description: The Z shell, the login shell of Silica users.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-autotools.bst

depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:bootstrap/ncurses.bst
- freedesktop-sdk.bst:bootstrap/pcre2.bst

variables:
  conf-local: >-
    --enable-etcdir=%{sysconfdir}
    --enable-multibyte
    --enable-pcre
    --enable-cap
    --disable-gdbm
    --with-tcsetpgrp
    --with-term-lib='tinfow ncursesw'
  # Programs, modules and functions only: no info/html documentation.
  make-install: >-
    make -j1 DESTDIR="%{install-root}"
    install.bin install.modules install.fns

config:
  install-commands:
    (>):
    - |
      # zsh installs a versioned hard link; the stripper refuses those.
      rm -f "%{install-root}%{bindir}"/zsh-*

public:
  initial-script:
    script: |
      #!/bin/bash
      sysroot="${1}"
      mkdir -p "${sysroot}/etc"
      for s in /bin/zsh /usr/bin/zsh; do
        grep -qx "${s}" "${sysroot}/etc/shells" 2>/dev/null ||
          echo "${s}" >> "${sysroot}/etc/shells"
      done

sources:
- kind: tar
  url: zsh:zsh-5.9.2.tar.xz
  ref: 36fa734374b44783582cec09bcd67822e2f992c779ec1624ab5596df078d2f81
