kind: script
description: |
  The Silica disk image (for QEMU and for writing to a USB stick) and the
  update payload served to systemd-sysupdate.

build-depends:
- freedesktop-sdk.bst:vm/deploy-tools.bst
- freedesktop-sdk.bst:components/xz.bst
- silica/system/config.bst
- filename: silica/image/usr.bst
  config:
    location: /usr-image
- filename: silica/image/uki.bst
  config:
    location: /uki

environment:
  XZFLAGS: '-T%{max-jobs}'

environment-nocache:
- XZFLAGS

variables:
  uuidnamespace: 5f1c1a6e-3b8f-4d61-9a3c-7d0c2f8e5b11
  build-root: /genimage

config:
  commands:
  - |
    set -eu
    v='%{silica-version}'
    . /usr-image/vars
    mkdir -p %{build-root}/esp/EFI/BOOT %{build-root}/esp/EFI/Linux %{build-root}/tmp
    cp "/uki/silica_${v}.efi" "%{build-root}/esp/EFI/Linux/silica_${v}.efi"
    # Fallback path for firmware without (or with lost) boot entries.
    cp "/uki/silica_${v}.efi" %{build-root}/esp/EFI/BOOT/BOOTX64.EFI
    # Slot sizes come from the repart definitions, so they always agree.
    size() { sed -n 's/^SizeMinBytes=//p' "/usr/lib/repart.d/$1"; }
    usr_size="$(size 20-usr-a.conf)"
    verity_size="$(size 21-usr-verity-a.conf)"
    sig_size="$(size 22-usr-verity-sig-a.conf)"
    efi_id="$(uuidgen -s --namespace '%{uuidnamespace}' --name efi | cut -c1-8)"
    cat >%{build-root}/genimage.cfg <<EOF2
    image esp.img {
        vfat {
            extraargs = "--invariant -F32 -i${efi_id} -n SILICA"
        }
        mountpoint = "/esp"
        size = 512M
        temporary = true
    }
    image silica.img {
        hdimage {
            align = 1M
            partition-table-type = "gpt"
            disk-uuid = "$(uuidgen -s --namespace '%{uuidnamespace}' --name disk)"
        }
        partition esp {
            image = "esp.img"
            partition-type-uuid = "U"
            partition-uuid = "$(uuidgen -s --namespace '%{uuidnamespace}' --name esp)"
        }
        partition silica_usr_${v} {
            image = "/usr-image/usr.erofs"
            partition-type-uuid = "8484680c-9521-48c6-9c11-b0720656f69e"
            partition-uuid = "${usr_uuid}"
            size = ${usr_size}
        }
        partition silica_usr_verity_${v} {
            image = "/usr-image/usr.verity"
            partition-type-uuid = "77ff5f63-e7b6-4633-acf4-1565b864c0e6"
            partition-uuid = "${verity_uuid}"
            size = ${verity_size}
        }
        partition silica_usr_verity_sig_${v} {
            image = "/usr-image/usr.verity-sig"
            partition-type-uuid = "e7bb33fb-06cf-4e81-8273-e543b413e2e2"
            partition-uuid = "$(uuidgen -s --namespace '%{uuidnamespace}' --name "sig-${v}")"
            size = ${sig_size}
        }
    }
    config {
        tmppath = "%{build-root}/tmp"
        outputpath = "%{install-root}"
    }
    EOF2
    mkdir -p /esp
    cp -a %{build-root}/esp/. /esp/
    genimage --config %{build-root}/genimage.cfg --rootpath /

  - |
    set -eu
    v='%{silica-version}'
    . /usr-image/vars
    u="%{install-root}/update"
    mkdir -p "${u}"
    xz ${XZFLAGS} -c /usr-image/usr.erofs >"${u}/silica_${v}_${usr_uuid}.usr.erofs.xz"
    xz ${XZFLAGS} -c /usr-image/usr.verity >"${u}/silica_${v}_${verity_uuid}.usr.verity.xz"
    xz ${XZFLAGS} -c /usr-image/usr.verity-sig >"${u}/silica_${v}.usr.verity-sig.xz"
    xz ${XZFLAGS} -c "/uki/silica_${v}.efi" >"${u}/silica_${v}.efi.xz"
    (cd "${u}" && sha256sum * >SHA256SUMS)
    ls -la "${u}"
