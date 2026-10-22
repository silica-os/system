kind: meson
description: |
  pam_selinux, built from the same Linux-PAM as freedesktop-sdk's
  components/linux-pam-base.bst, which has -Dselinux=disabled. systemd with
  SELinux puts it in its PAM files (systemd-user, systemd-run0): without it,
  no user manager starts. silica/image/usr.bst copies it into the image.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-meson.bst
- freedesktop-sdk.bst:components/flex.bst
- freedesktop-sdk.bst:components/audit.bst
- freedesktop-sdk.bst:components/libeconf.bst
- freedesktop-sdk.bst:bootstrap/libselinux.bst

variables:
  meson-local: >-
    -Dpam_lastlog=enabled
    -Dexamples=false
    -Dlogind=disabled
    -Dnis=disabled
    -Dselinux=enabled
    -Ddocs=disabled
    -Delogind=disabled
    -Dpwaccess=disabled

config:
  install-commands:
    (>):
    - |
      # Only the SELinux modules; the rest comes from freedesktop-sdk.
      cd '%{install-root}'
      find . -mindepth 1 \( -type f -o -type l \) \
        ! -path '.%{libdir}/security/pam_selinux.so' \
        ! -path '.%{libdir}/security/pam_sepermit.so' -delete
      find . -mindepth 1 -depth -type d -empty -delete
      test -f '.%{libdir}/security/pam_selinux.so'

sources:
- kind: git_repo
  url: github:linux-pam/linux-pam.git
  track: v*
  ref: v1.7.3-0-g28fce1fd55b003fec90f34e5bc785e9439b216f2
