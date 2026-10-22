# Silica's kernel configuration, applied after freedesktop-sdk's
# fdsdk-config.sh. See docs/base-system.md, section 2.

# Options that may not exist in every kernel release.
try_module() {
  scripts/config --keep-case -m "${1}"
}
try_enable() {
  scripts/config --keep-case -e "${1}"
}

# Module signing is mandatory: only modules signed with Silica's key load.
enable MODULE_SIG_FORCE

# Lockdown in integrity mode, whatever the command line says.
enable SECURITY_LOCKDOWN_LSM
enable SECURITY_LOCKDOWN_LSM_EARLY
remove LOCK_DOWN_KERNEL_FORCE_NONE
enable LOCK_DOWN_KERNEL_FORCE_INTEGRITY

# Integrity Policy Enforcement, fed by dm-verity and fs-verity signatures.
enable FS_VERITY
enable FS_VERITY_BUILTIN_SIGNATURES
enable SECURITY_IPE
enable IPE_PROP_DM_VERITY
enable IPE_PROP_DM_VERITY_SIGNATURE
enable IPE_PROP_FS_VERITY
enable IPE_PROP_FS_VERITY_BUILTIN_SIG

# The LSM stack. SELinux is built in; its policy comes later.
enable SECURITY_YAMA
enable SECURITY_LANDLOCK
enable SECURITY_SELINUX
value_str LSM "landlock,lockdown,yama,integrity,ipe,selinux,bpf"

# Memory hardening.
enable INIT_STACK_ALL_ZERO
enable INIT_ON_ALLOC_DEFAULT_ON
enable INIT_ON_FREE_DEFAULT_ON
enable RANDOMIZE_KSTACK_OFFSET
enable RANDOMIZE_KSTACK_OFFSET_DEFAULT
remove SLAB_MERGE_DEFAULT
enable SLAB_FREELIST_RANDOM
enable SLAB_FREELIST_HARDENED
enable SHUFFLE_PAGE_ALLOCATOR
enable HARDENED_USERCOPY
enable FORTIFY_SOURCE
enable STACKPROTECTOR_STRONG
enable BUG_ON_DATA_CORRUPTION
enable LIST_HARDENED

# Legacy interfaces that only widen the attack surface.
# /dev/mem stays (STRICT_DEVMEM); lockdown denies it to everyone.
remove DEVPORT
remove LEGACY_TIOCSTI
remove LEGACY_VSYSCALL_XONLY
remove LEGACY_VSYSCALL_EMULATE
enable LEGACY_VSYSCALL_NONE
remove X86_IOPL_IOPERM
remove MODIFY_LDT_SYSCALL
remove USELIB
remove IA32_EMULATION
remove X86_X32_ABI
remove KEXEC
enable KEXEC_FILE
enable KEXEC_SIG
enable KEXEC_SIG_FORCE
enable BPF_UNPRIV_DEFAULT_OFF
remove PROC_KCORE
remove HIBERNATION

# Drivers fdsdk leaves out.
module HFSPLUS_FS
try_module DRM_NOVA
try_module NOVA_CORE
try_module VIDEO_INTEL_IPU6
try_module INTEL_SKL_INT3472
try_enable HID_SENSOR_HUB
try_module HID_SENSOR_ALS
try_module HID_SENSOR_ACCEL_3D
try_module HID_SENSOR_GYRO_3D
try_module HID_SENSOR_DEVICE_ROTATION

# The system image is erofs on dm-verity; data is btrfs on dm-crypt.
module EROFS_FS
module BTRFS_FS
module DM_CRYPT
module DM_VERITY
enable DM_VERITY_VERIFY_ROOTHASH_SIG
