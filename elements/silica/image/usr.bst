kind: script
description: |
  The sealed system image: /usr as erofs, its dm-verity hash tree, and the
  root hash signed with Silica's system key (checked by the kernel).

build-depends:
- freedesktop-sdk.bst:components/fakecap.bst
- silica/components/erofs-utils-selinux.bst
- freedesktop-sdk.bst:components/cryptsetup.bst
- freedesktop-sdk.bst:components/util-linux.bst
- freedesktop-sdk.bst:components/openssl.bst
- freedesktop-sdk.bst:vm/deploy-tools.bst
- silica/image/initial-scripts.bst
- filename: silica/system/filesystem.bst
  config:
    location: /sysroot
- filename: silica/image/keys.bst
  config:
    location: /keys
- filename: silica/components/systemd-selinux.bst
  config:
    location: /systemd-selinux
- filename: silica/components/pam-selinux.bst
  config:
    location: /pam-selinux

environment:
  LD_PRELOAD: /usr/libexec/fakecap/fakecap.so
  FAKECAP_DB: /fakecap

variables:
  uuidnamespace: 5f1c1a6e-3b8f-4d61-9a3c-7d0c2f8e5b11
  salt: 3c6b1f0e8a4d2c7b9e5f1a3d6c8b0e2f4a7c9d1b3e5f7a9c0d2e4f6a8b0c1d3e

config:
  commands:
  - mkdir -p /fakecap

  - |
    # systemd with SELinux, over freedesktop-sdk's build of the same version.
    cp -a /systemd-selinux/. /sysroot/
    cp -a /pam-selinux/. /sysroot/
    rm -rf /sysroot/usr/lib/debug

  - |
    prepare-image.sh --sysroot /sysroot --seed "%{uuidnamespace}" --noboot --noroot >/dev/null

  - |
    # The code integrity policy, signed so the kernel accepts it.
    openssl smime -sign -binary -noattr -nodetach -nosmimecap -nocerts -outform DER \
      -signer /keys/system.crt -inkey /keys/system.key \
      -in /sysroot/usr/lib/silica/ipe/silica.pol \
      -out /sysroot/usr/lib/silica/ipe/silica.p7s

  - |
    # The key systemd-sysupdate checks the update manifests with (older
    # systemd versions read the .gpg name).
    install -Dm644 /keys/update-pubring.pgp /sysroot/usr/lib/systemd/import-pubring.pgp
    ln -sf import-pubring.pgp /sysroot/usr/lib/systemd/import-pubring.gpg

  - |
    # Factory defaults for /etc; the data partition starts with an empty /etc.
    mkdir -p /sysroot/usr/share/factory/etc
    cp -a /sysroot/etc/. /sysroot/usr/share/factory/etc/
    rm -f /sysroot/usr/share/factory/etc/machine-id

  - |
    # Labelled from the policy's file contexts, so /usr carries its labels.
    mkfs.erofs -zlz4hc,12 -T "${SOURCE_DATE_EPOCH}" --all-time \
      --file-contexts=/sysroot/usr/lib/selinux/silica/contexts/files/file_contexts \
      --mount-point=/usr \
      -U "$(uuidgen -s --namespace '%{uuidnamespace}' --name usr)" \
      /usr.erofs /sysroot/usr

  - |
    veritysetup format --salt='%{salt}' \
      --uuid="$(uuidgen -s --namespace '%{uuidnamespace}' --name verity)" \
      /usr.erofs /usr.verity | sed -n 's/^Root hash:[[:space:]]*//p' >/roothash
    test -s /roothash

  - |
    # PKCS#7 detached signature of the hex root hash, as systemd and the
    # kernel's dm-verity root_hash_sig expect it.
    printf %s "$(cat /roothash)" >/roothash.txt
    openssl smime -sign -binary -noattr -nocerts -outform DER \
      -signer /keys/system.crt -inkey /keys/system.key \
      -in /roothash.txt -out /roothash.p7s
    fp="$(openssl x509 -in /keys/system.crt -outform DER | sha256sum | cut -d' ' -f1)"
    printf '{"rootHash":"%s","certificateFingerprint":"%s","signature":"%s"}' \
      "$(cat /roothash)" "${fp}" "$(base64 -w0 /roothash.p7s)" >/usr.verity-sig
    truncate -s 4096 /usr.verity-sig

  - |
    hash="$(cat /roothash)"
    data_uuid="$(echo "${hash}" | sed -E 's/^(.{8})(.{4})(.{4})(.{4})(.{12}).*/\1-\2-\3-\4-\5/')"
    verity_uuid="$(echo "${hash}" | sed -E 's/.*(.{8})(.{4})(.{4})(.{4})(.{12})$/\1-\2-\3-\4-\5/')"
    out="%{install-root}"
    mv /usr.erofs "${out}/usr.erofs"
    mv /usr.verity "${out}/usr.verity"
    mv /usr.verity-sig "${out}/usr.verity-sig"
    cp /roothash "${out}/roothash"
    base64 -w0 /roothash.p7s >"${out}/roothash.p7s.b64"
    cat >"${out}/vars" <<EOF
    usr_uuid=${data_uuid}
    verity_uuid=${verity_uuid}
    EOF
    du -h "${out}"/usr.*
