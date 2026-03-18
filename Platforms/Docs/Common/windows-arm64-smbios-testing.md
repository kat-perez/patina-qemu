# Windows ARM64 SMBIOS OS-side Verification

This guide documents how to set up and run OS-side SMBIOS verification on a Windows ARM64 guest under QEMU SBSA
emulation. This is used to verify that SMBIOS tables published by the firmware are correctly visible to the Windows
operating system via WMI/CIM.

## Overview

The UEFI Shell `smbiosview` command can verify SMBIOS tables at the firmware level, but OS-side verification ensures
that Windows actually parses and exposes the tables correctly. This guide covers:

1. Creating a Windows ARM64 QCOW2 overlay image
2. Injecting a Group Policy startup script that queries all SMBIOS types
3. Booting QEMU SBSA with the patched firmware
4. Extracting and comparing the results

## Prerequisites

- Linux host (x86_64) with QEMU installed (aarch64 system emulation support)
- A Windows 11 ARM64 image (ISO or VHDX)
- `qemu-nbd`, `ntfs-3g`, `ntfsfix` packages installed (`apt install qemu-utils ntfs-3g`)
- The patina-qemu firmware already built for SBSA (`Build/QemuSbsaPkg/DEBUG_GCC5/FV/`)

## Step 1: Prepare the Windows Disk Image

Create a QCOW2 overlay so you don't modify the original Windows image:

```bash
# Create a working directory
mkdir -p /tmp/smbios-test
cd /tmp/smbios-test

# If starting from a VHDX, first convert to qcow2
qemu-img convert -f vhdx -O qcow2 /path/to/Windows11_ARM64.VHDX windows_base.qcow2

# Create an overlay so the base image is never modified
qemu-img create -f qcow2 -b windows_base.qcow2 -F qcow2 smbios_verify.qcow2
```

## Step 2: First Boot - Windows Setup

The first boot completes Windows OOBE (Out of Box Experience). This is very slow under emulation but only needs to be
done once — the overlay preserves the installed state for subsequent boots.

```bash
cd /path/to/patina-qemu

python build_and_run_rust_binary.py \
  --crate-patch ../patina \
  --toolchain GCC5 -p SBSA \
  --qemu-path /path/to/qemu-system-aarch64 \
  --core-count 4 \
  --os /tmp/smbios-test/smbios_verify.qcow2 \
  --headless \
  --serial-port 50001 \
  --monitor-port 50002
```

> **Important:** `--core-count 4` must match `PcdCoreCount` in the build. A mismatch causes an ASSERT crash.

> **Important:** Always include `--monitor-port` so you can take screenshots to check boot progress:
> ```bash
> echo "screendump /tmp/screen.ppm" | nc -w 3 127.0.0.1 50002
> ```

The first boot takes a long time (multiple hours) for OOBE to complete. Monitor CPU usage — when it drops below ~50%
and stays there, Windows has likely reached the desktop or lock screen. Kill QEMU once setup is complete.

## Step 3: Inject the SMBIOS Query Script

After the first boot, mount the Windows partition and inject a Group Policy Machine startup script. This script runs
automatically at every boot before user login.

```bash
# Connect the overlay via NBD
sudo modprobe nbd
sudo qemu-nbd -c /dev/nbd0 /tmp/smbios-test/smbios_verify.qcow2
sudo partx -a /dev/nbd0

# Fix dirty NTFS (always needed after killing QEMU)
sudo ntfsfix /dev/nbd0p3

# Mount read-write
sudo mkdir -p /tmp/smbios-test/win_mnt
sudo ntfs-3g -o rw /dev/nbd0p3 /tmp/smbios-test/win_mnt
```

Create the Group Policy startup script directory and script:

```bash
sudo mkdir -p "/tmp/smbios-test/win_mnt/Windows/System32/GroupPolicy/Machine/Scripts/Startup"
```

Write the following to `smbios_startup.cmd`:

```cmd
@echo off
echo ============================================ > C:\smbios_gp_output.txt
echo SMBIOS Data (Group Policy Startup) - %DATE% %TIME% >> C:\smbios_gp_output.txt
echo ============================================ >> C:\smbios_gp_output.txt

powershell -NoProfile -Command "Write-Output '--- Type 0: BIOS ---'; Get-CimInstance Win32_BIOS | Format-List *; Write-Output ''; Write-Output '--- Type 1: System ---'; Get-CimInstance Win32_ComputerSystem | Format-List *; Write-Output ''; Write-Output '--- Type 2: Baseboard ---'; Get-CimInstance Win32_BaseBoard | Format-List *; Write-Output ''; Write-Output '--- Type 3: System Enclosure ---'; Get-CimInstance Win32_SystemEnclosure | Format-List *; Write-Output ''; Write-Output '--- Type 4: Processor ---'; Get-CimInstance Win32_Processor | Format-List *; Write-Output ''; Write-Output '--- Type 7: Cache Memory ---'; Get-CimInstance Win32_CacheMemory | Format-List *; Write-Output ''; Write-Output '--- Type 16: Physical Memory Array ---'; Get-CimInstance Win32_PhysicalMemoryArray | Format-List *; Write-Output ''; Write-Output '--- Type 17: Memory Device ---'; Get-CimInstance Win32_PhysicalMemory | Format-List *; Write-Output ''; Write-Output ('--- Script completed at: ' + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fff') + ' ---')" >> C:\smbios_gp_output.txt 2>&1

echo DONE %DATE% %TIME% > C:\smbios_gp_done.txt
```

> **Critical:** Use a **single** `powershell -NoProfile -Command` invocation for all queries. Under ARM64 emulation,
> each separate PowerShell process pays the full .NET JIT compilation cost (~20+ min each). A single invocation
> completes all 8 queries in ~2 minutes, while 8 separate invocations take ~3 hours.

You also need a `scripts.ini` file to register the startup script with Group Policy:

```bash
sudo mkdir -p "/tmp/smbios-test/win_mnt/Windows/System32/GroupPolicy/Machine/Scripts"
```

Write the following to `scripts.ini`:

```ini
[Startup]
0CmdLine=C:\Windows\System32\GroupPolicy\Machine\Scripts\Startup\smbios_startup.cmd
0Parameters=
```

Create a `GptTmpl.inf` file so Group Policy processes the scripts:

```bash
sudo mkdir -p "/tmp/smbios-test/win_mnt/Windows/System32/GroupPolicy/Machine/Microsoft/Windows NT/SecEdit"
```

Write the following to `GptTmpl.inf`:

```ini
[Unicode]
Unicode=yes
[Version]
signature="$CHICAGO$"
Revision=1
```

Ensure the `gpt.ini` file exists at the Group Policy root:

Write the following to `/tmp/smbios-test/win_mnt/Windows/System32/GroupPolicy/gpt.ini`:

```ini
[General]
gPCMachineExtensionNames=[{42B5FAAE-6536-11D2-AE5A-0000F87571E3}{40B6664F-4972-11D1-A7CA-0000F87571E3}]
Version=65537
```

Unmount when done:

```bash
sudo umount /tmp/smbios-test/win_mnt
sudo qemu-nbd -d /dev/nbd0
```

## Step 4: Save the Variable Store

After the first successful boot where Windows detects the boot device, save the UEFI variable store. This preserves
the Boot Manager entry so subsequent boots go straight to Windows:

```bash
cp Build/QemuSbsaPkg/DEBUG_GCC5/FV/SECURE_FLASH0.fd /tmp/smbios-test/SECURE_FLASH0_verify.fd
```

Before each subsequent boot, restore it:

```bash
cp /tmp/smbios-test/SECURE_FLASH0_verify.fd Build/QemuSbsaPkg/DEBUG_GCC5/FV/SECURE_FLASH0.fd
```

## Step 5: Boot and Collect Results

```bash
# Restore the saved variable store
cp /tmp/smbios-test/SECURE_FLASH0_verify.fd \
   Build/QemuSbsaPkg/DEBUG_GCC5/FV/SECURE_FLASH0.fd

# Boot QEMU (--no-build skips firmware compilation)
python build_and_run_rust_binary.py \
  --crate-patch ../patina \
  --toolchain GCC5 -p SBSA \
  --qemu-path /path/to/qemu-system-aarch64 \
  --core-count 4 \
  --os /tmp/smbios-test/smbios_verify.qcow2 \
  --headless \
  --serial-port 50001 \
  --monitor-port 50002 \
  --no-build
```

### Warm Boot Timing

With an existing Windows install (overlay), the boot-to-script-complete timeline is:

| Phase | Time |
|-------|------|
| QEMU start to lock screen | ~2-3 minutes |
| Lock screen to GP script start | ~1 minute |
| All CIM queries (single PS invocation) | ~2 minutes |
| **Total: QEMU start to script done** | **~5 minutes** |

Monitor progress using screenshots via the QEMU monitor:

```bash
# Take a screenshot
echo "screendump /tmp/screen.ppm" | nc -w 3 127.0.0.1 50002

# Convert to PNG for viewing (requires netpbm)
pnmtopng /tmp/screen.ppm > /tmp/screen.png
```

After ~5-10 minutes, kill QEMU and extract results:

```bash
kill $(pgrep qemu-system-aar)
```

## Step 6: Extract and Verify Results

```bash
# Connect and mount
sudo modprobe nbd
sudo qemu-nbd -c /dev/nbd0 /tmp/smbios-test/smbios_verify.qcow2
sudo partx -a /dev/nbd0
sudo ntfsfix /dev/nbd0p3
sudo ntfs-3g -o ro /dev/nbd0p3 /tmp/smbios-test/win_mnt

# Check that the script completed
cat /tmp/smbios-test/win_mnt/smbios_gp_done.txt

# Read the SMBIOS data
cat /tmp/smbios-test/win_mnt/smbios_gp_output.txt

# Save a copy
cp /tmp/smbios-test/win_mnt/smbios_gp_output.txt /tmp/smbios-test/smbios_results.txt

# Clean up
sudo umount /tmp/smbios-test/win_mnt
sudo qemu-nbd -d /dev/nbd0
```

### What to Verify

Compare the OS-side output against `smbiosview` output from the UEFI Shell:

| SMBIOS Type | WMI/CIM Class | Key Fields to Check |
|-------------|---------------|---------------------|
| Type 0 (BIOS) | `Win32_BIOS` | Manufacturer, SMBIOSBIOSVersion |
| Type 1 (System) | `Win32_ComputerSystem` | Manufacturer, Model |
| Type 2 (Baseboard) | `Win32_BaseBoard` | Manufacturer, Product |
| Type 3 (Enclosure) | `Win32_SystemEnclosure` | Manufacturer, ChassisTypes |
| Type 4 (Processor) | `Win32_Processor` | Name, NumberOfCores |
| Type 7 (Cache) | `Win32_CacheMemory` | InstalledSize, Level, Purpose |
| Type 16 (Memory Array) | `Win32_PhysicalMemoryArray` | MaxCapacity, MemoryDevices |
| Type 17 (Memory Device) | `Win32_PhysicalMemory` | Capacity, Speed, Manufacturer |

## Re-running Tests

To re-run after firmware changes, delete old output before booting:

```bash
# Mount read-write
sudo ntfs-3g -o rw /dev/nbd0p3 /tmp/smbios-test/win_mnt

# Delete old output
sudo rm -f /tmp/smbios-test/win_mnt/smbios_gp_output.txt \
           /tmp/smbios-test/win_mnt/smbios_gp_done.txt

# Unmount
sudo umount /tmp/smbios-test/win_mnt
sudo qemu-nbd -d /dev/nbd0
```

Then repeat Steps 5 and 6.

## Troubleshooting

### NTFS mount fails with "unclean file system"

Always run `ntfsfix` before mounting after killing QEMU:

```bash
sudo ntfsfix /dev/nbd0p3
```

### `wmic` commands fail

`wmic` is deprecated and removed on recent Windows 11 ARM64 builds. Use PowerShell `Get-CimInstance` instead.

### Script takes hours instead of minutes

Ensure all CIM queries are in a **single** `powershell -NoProfile -Command` invocation. Each separate PowerShell
process startup takes ~20+ minutes under ARM64 emulation due to .NET JIT compilation.

### Windows stuck at lock screen

The GP Machine startup script runs before user login, so the lock screen is expected. The script executes in the
background while the lock screen is displayed. You do not need to log in.

### QEMU crashes with ASSERT

Ensure `--core-count` matches the `PcdCoreCount` value used during the firmware build (typically 4).

### Black screen on screendump

The display may turn off after the lock screen timeout. Send a keypress via the monitor to wake it:

```bash
echo "sendkey spc" | nc -w 2 127.0.0.1 50002
```

Then take the screenshot again.
