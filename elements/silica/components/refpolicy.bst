kind: manual
description: |
  Silica's SELinux policy: the reference policy, built monolithic as the
  "silica" policy (MCS, systemd). It lives in /usr/lib/selinux/silica, sealed
  with the rest of /usr; /etc/selinux/silica points there, so every update
  brings its policy. /etc/selinux/config picks the mode.

build-depends:
- freedesktop-sdk.bst:public-stacks/buildsystem-make.bst
- freedesktop-sdk.bst:components/m4.bst
- freedesktop-sdk.bst:components/gawk.bst
- freedesktop-sdk.bst:components/python3.bst
- freedesktop-sdk.bst:components/util-linux.bst  # getopt, for support/gentemplates.sh
- silica/components/selinux-utils.bst

depends:
- silica/components/selinux-utils.bst

variables:
  policy-args: >-
    NAME=silica
    TYPE=mcs
    MONOLITHIC=y
    SYSTEMD=y
    UBAC=n
    UNK_PERMS=allow
    DIRECT_INITRC=n
    CHECKPOLICY=checkpolicy
    SETFILES=setfiles
    QUIET=y

config:
  configure-commands:
  - |
    # Silica's paths: homes live in /Users, and the policy in /usr/lib/selinux.
    cat >>config/file_contexts.subs_dist <<'EOF'
    /Users /home
    /usr/lib/selinux /etc/selinux
    EOF
  - make %{policy-args} conf

  build-commands:
  - make %{policy-args} policy

  install-commands:
  - make %{policy-args} DESTDIR='%{install-root}' install
  - |
    mkdir -p '%{install-root}%{indep-libdir}/selinux'
    mv '%{install-root}%{sysconfdir}/selinux/silica' '%{install-root}%{indep-libdir}/selinux/silica'
    ln -s ../..%{indep-libdir}/selinux/silica '%{install-root}%{sysconfdir}/selinux/silica'
  - |
    # Permissive until the policy is verified with Silica's services.
    cat >'%{install-root}%{sysconfdir}/selinux/config' <<'EOF'
    SELINUX=permissive
    SELINUXTYPE=silica
    EOF

sources:
- kind: git_repo
  url: github:SELinuxProject/refpolicy.git
  track: RELEASE_2_*
  ref: RELEASE_2_20260922-0-gdfe5f0b23ebb640151e9222e0a72b35deac022e7
