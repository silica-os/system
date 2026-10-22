kind: meson
description: |
  systemd built with SELinux. freedesktop-sdk builds the same version with
  -Dselinux=disabled; silica/image/usr.bst copies these files over it.

(@):
- freedesktop-sdk.bst:elements/include/systemd.yml

build-depends:
- freedesktop-sdk.bst:public-stacks/runtime-minimal.bst
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:bootstrap/sed.bst
- freedesktop-sdk.bst:bootstrap/gawk.bst
- freedesktop-sdk.bst:bootstrap/grep.bst
- freedesktop-sdk.bst:bootstrap/diffutils.bst
- freedesktop-sdk.bst:bootstrap/libselinux.bst
- freedesktop-sdk.bst:components/audit.bst
- freedesktop-sdk.bst:components/gperf.bst
- freedesktop-sdk.bst:components/m4.bst
- freedesktop-sdk.bst:components/libapparmor.bst
- freedesktop-sdk.bst:components/libcap.bst
- freedesktop-sdk.bst:components/libgcrypt.bst
- freedesktop-sdk.bst:components/libgpg-error.bst
- freedesktop-sdk.bst:components/libseccomp.bst
- freedesktop-sdk.bst:components/lz4.bst
- freedesktop-sdk.bst:components/zstd.bst
- freedesktop-sdk.bst:components/util-linux-libfdisk.bst
- freedesktop-sdk.bst:components/util-linux-libs.bst
- freedesktop-sdk.bst:components/util-linux-full.bst
- freedesktop-sdk.bst:components/libarchive.bst
- freedesktop-sdk.bst:components/linux-pam.bst
- freedesktop-sdk.bst:components/kmod.bst
- freedesktop-sdk.bst:components/python3-pyelftools.bst
- freedesktop-sdk.bst:components/libxslt.bst
- freedesktop-sdk.bst:components/docbook-xsl.bst
- freedesktop-sdk.bst:components/_private/cryptsetup-lvm2-stage1.bst
- freedesktop-sdk.bst:components/p11-kit.bst
- freedesktop-sdk.bst:components/libfido2.bst
- freedesktop-sdk.bst:components/libidn2.bst
- freedesktop-sdk.bst:components/openssl.bst
- freedesktop-sdk.bst:components/python3-pefile.bst
- freedesktop-sdk.bst:components/python3-jinja2.bst
- freedesktop-sdk.bst:components/apparmor.bst
- freedesktop-sdk.bst:components/tpm2-tss.bst
- freedesktop-sdk.bst:components/curl.bst
- freedesktop-sdk.bst:components/libqrencode.bst
- freedesktop-sdk.bst:components/iptables.bst
- freedesktop-sdk.bst:components/libxkbcommon.bst
- freedesktop-sdk.bst:components/libmicrohttpd.bst
- freedesktop-sdk.bst:components/bpf-maybe.bst
- freedesktop-sdk.bst:components/libbpf.bst
- freedesktop-sdk.bst:components/llvm.bst

variables:
  # The same options as freedesktop-sdk's components/_private/systemd-base.bst,
  # except -Dselinux, so the files match the build they replace.
  meson-local: >-
    -Dmode=release
    -Dsysvinit-path=%{sysconfdir}/init.d
    -Daudit=enabled
    -Dseccomp=enabled
    -Dsystem-uid-max=999
    -Dsystem-gid-max=999
    -Dusers-gid=100
    -Dopenssl=enabled
    -Dpam=enabled
    -Dbootloader=true
    -Defi=true
    -Dfirstboot=true
    -Dzlib=enabled
    -Dzstd=enabled
    -Dbzip2=enabled
    -Dxz=enabled
    -Dlz4=enabled
    -Ddefault-dnssec=no
    -Didn=true
    -Drepart=enabled
    -Dman=disabled
    -Dhtml=disabled
    -Dlibcryptsetup=enabled
    -Dlibcryptsetup-plugins=enabled
    -Dp11kit=enabled
    -Dlibfido2=enabled
    -Dapparmor=enabled
    -Dtpm=true
    -Dtpm2=enabled
    -Dsbat-distro=freedesktop-sdk
    -Dsbat-distro-generation=1
    -Dsbat-distro-summary="Freedesktop SDK"
    -Dsbat-distro-url=https://gitlab.com/freedesktop-sdk/freedesktop-sdk/
    -Dversion-tag="$(git describe --abbrev=7 | sed "s/^v//")"
    -Dqrencode=enabled
    -Dxkbcommon=enabled
    -Dlibidn2=enabled
    -Dlibiptc=enabled
    -Dxenctrl=disabled
    -Dbpf-framework=enabled
    -Dgnutls=disabled
    -Dglib=disabled
    -Ddbus=disabled
    -Dukify=true
    -Dkernel-install=true
    -Dvmlinux-h=provided
    -Dvmlinux-h-path='%{includedir}/%{gcc_triplet}/linux/vmlinux.h'
    -Dselinux=enabled
    -Dsysupdated=enabled

config:
  install-commands:
    (>):
    - |
      # Only what replaces freedesktop-sdk's files in the image.
      rm -rf '%{install-root}%{includedir}' '%{install-root}%{libdir}/pkgconfig' \
        '%{install-root}%{datadir}/pkgconfig' '%{install-root}%{sysconfdir}' \
        '%{install-root}%{localstatedir}' '%{install-root}%{datadir}/doc'
