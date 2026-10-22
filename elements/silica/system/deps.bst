kind: stack
description: |
  Everything in the Silica base system. See docs/base-system.md, section 3.

depends:
# Core userland, init and login.
- freedesktop-sdk.bst:public-stacks/runtime-gnu.bst
- freedesktop-sdk.bst:components/systemd.bst
- freedesktop-sdk.bst:components/gnupg.bst  # systemd-sysupdate checks SHA256SUMS.gpg with gpg
- freedesktop-sdk.bst:components/dbus.bst
- freedesktop-sdk.bst:components/dbus-broker.bst
- freedesktop-sdk.bst:components/linux-pam.bst
- freedesktop-sdk.bst:components/shadow.bst
- freedesktop-sdk.bst:components/polkit.bst
- freedesktop-sdk.bst:components/audit.bst
- freedesktop-sdk.bst:components/kmod.bst
- freedesktop-sdk.bst:components/util-linux-full.bst
- freedesktop-sdk.bst:components/procps.bst
- freedesktop-sdk.bst:components/kbd.bst
- freedesktop-sdk.bst:components/less.bst
- freedesktop-sdk.bst:components/nano.bst
- freedesktop-sdk.bst:components/bash-config.bst
- freedesktop-sdk.bst:components/bash-completion.bst
- freedesktop-sdk.bst:components/hosts.bst
- freedesktop-sdk.bst:components/tzdata.bst
- freedesktop-sdk.bst:components/locales.bst
- freedesktop-sdk.bst:components/keyutils.bst
# Storage and encryption.
- freedesktop-sdk.bst:components/cryptsetup.bst
# device-mapper udev rules: without them cryptsetup waits forever on udev.
- freedesktop-sdk.bst:components/lvm2.bst
- freedesktop-sdk.bst:components/tpm2-tss.bst
- freedesktop-sdk.bst:components/tpm2-tools.bst
- freedesktop-sdk.bst:components/btrfs-progs.bst
- freedesktop-sdk.bst:components/e2fsprogs.bst
- freedesktop-sdk.bst:components/dosfstools.bst
- freedesktop-sdk.bst:components/exfat-progs.bst
- freedesktop-sdk.bst:components/erofs-utils.bst
# Hardware: kernel, firmware, buses.
- silica/kernel/modules.bst
- freedesktop-sdk.bst:components/linux-firmware.bst
- freedesktop-sdk.bst:components/sof-firmware.bst
- freedesktop-sdk.bst:components/wireless-regdb-bin.bst
- freedesktop-sdk.bst:components/usbutils.bst
- freedesktop-sdk.bst:components/pciutils.bst
- freedesktop-sdk.bst:components/fwupd.bst
- freedesktop-sdk.bst:components/upower.bst
- freedesktop-sdk.bst:components/iio-sensor-proxy.bst
- freedesktop-sdk.bst:components/libinput.bst
# Network.
- freedesktop-sdk.bst:components/networkmanager.bst
- freedesktop-sdk.bst:components/iwd.bst
- freedesktop-sdk.bst:components/modemmanager.bst
- freedesktop-sdk.bst:components/iproute2.bst
- freedesktop-sdk.bst:components/nftables.bst
- freedesktop-sdk.bst:components/avahi.bst
- freedesktop-sdk.bst:components/curl.bst
- freedesktop-sdk.bst:components/bluez.bst
# Audio, video, graphics.
- freedesktop-sdk.bst:components/pipewire.bst
- freedesktop-sdk.bst:components/wireplumber.bst
- freedesktop-sdk.bst:components/alsa-lib.bst
- freedesktop-sdk.bst:components/libcamera.bst
- freedesktop-sdk.bst:vm/mesa-default.bst
# Printing, location, fonts.
- freedesktop-sdk.bst:components/cups-daemon.bst
- freedesktop-sdk.bst:components/cups-filters.bst
- freedesktop-sdk.bst:components/geoclue.bst
- freedesktop-sdk.bst:components/noto-fonts.bst
- freedesktop-sdk.bst:components/noto-emoji.bst
# Silica's own recipes.
- silica/components/zsh.bst
- silica/components/thermald.bst
- silica/components/bolt.bst
- silica/components/power-profiles-daemon.bst
- silica/components/switcheroo-control.bst
- silica/components/libfprint.bst
- silica/components/fprintd.bst
- silica/components/espeak-ng.bst
- silica/components/speech-dispatcher.bst
- silica/components/brltty.bst
- silica/components/smartmontools.bst
- silica/components/nvme-cli.bst
- silica/components/sane-backends.bst
- silica/components/sane-airscan.bst
- silica/components/ipp-usb.bst
- silica/components/intel-microcode.bst
- silica/components/openvpn.bst
# Silica.
- silica/system/config.bst
- silica/system/tools.bst
