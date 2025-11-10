# patina-tianocore-dev
Repository to share co-developed code between Dell and ODP for use in both Patina and Tianocore builds

SPECIAL NOTE: This repository is currently licensed under the MIT license.  By submitting code to this repository,
you acknowledge that the changes contain no patentable content.  The changes will be made freely available to all
participants under the MIT license.  These changes may be moved later to a repository that is alternately licensed
(for example, under BSD-2-Clause-Patent License) but such a chance would be managed in accordance with the ODP
governance-managed RFC process.

## Repo 1: Public Patina SMBIOS component

[ODP patina-smbios - Branch: main](https://github.com/OpenDevicePartnership/patina-smbios)

This repo contains the official Patina SMBIOS module code and is a temporary location until
copyright information can be finalized then will move to the main Patina repo.

The code produces a Rust crate to be ingested into the Patina DXE core as a component and
does not produce a binary executable on it's own.  Testing is done in one of the other
following repos.


## Repo 2/3: Public repos for testing the Patina SMBIOS module

[ODP patina-dxe-core-qemu - Branch: feature/smbios-component](https://github.com/OpenDevicePartnership/patina-dxe-core-qemu/tree/feature/smbios-component)

This repo contains code to create a sample Patina DXE Core EFI driver that uses the patina-smbios component in a QEMU environment.  The branch ```feature/smbios-component``` is where changes are being made and which will eventually be merged into main.  Running ```cargo make q35``` will create the ```target\x86_64-unknown-uefi\debug\qemu_q35_dxe_core.efi``` file that can be used in the next repo (patina-qemu).

[ODP patina-qemu - Branch: main](https://github.com/OpenDevicePartnership/patina-qemu.git)

This is the normal QEMU build tree to test the Patina DXE Core.  The [Advanced Usage](https://github.com/OpenDevicePartnership/patina-qemu?tab=readme-ov-file#advanced-usage) section of the Readme.md file outlines the steps to add the qemu_q35_dxe_core.efi file for testing.


## Repo 4: Private QEMU UEFI build for testing the Patina SMBIOS module as a Tianocore driver

[PRIVATE patina-tianocore-dev - Branch: main](https://github.com/rogurr/patina-tianocore-dev)

This repo contains a copy of the patina-qemu tree with the following changes:
* Disable the Patina DXE core
* Enable the Tiano DXE core
* Add a ```/Platforms/PATINA_TIANOCORE_DEV/PatinaSmbiosDxe``` driver

The new driver is written in Rust and demonstrates how to use and publish normal Tianocore interfaces while using a Patina component.

### Building and Running with PatinaSmbiosDxe

The PatinaSmbiosDxe driver is a Rust-based UEFI DXE driver that wraps the Patina SMBIOS component to provide SMBIOS functionality within the Tianocore DXE environment. This driver demonstrates how to integrate Patina components with the standard Tianocore build system.

#### Prerequisites

Follow the setup instructions in [QemuReadme.md](./QemuReadme.md) to ensure all required tools are installed:
- Rust toolchain with `x86_64-unknown-uefi` target
- Python 3 with virtual environment support
- Stuart build tools (installed via pip-requirements.txt)
- QEMU for testing

#### Building PatinaSmbiosDxe

The PatinaSmbiosDxe driver is included in the standard QemuQ35Pkg build process as a Rust module. The driver is located at `Platforms/PATINA_TIANOCORE_DEV/PatinaSmbiosDxe/`.

**Build Steps:**

1. **Setup Python Virtual Environment:**
   ```bash
   # Windows
   python -m venv q35env
   q35env\Scripts\activate.bat

   # Linux/WSL
   python -m venv q35env
   source q35env/bin/activate
   ```

2. **Install Build Prerequisites:**
   ```bash
   pip install --upgrade -r pip-requirements.txt
   ```

3. **Run Stuart Setup:**
   ```bash
   stuart_setup -c Platforms/QemuQ35Pkg/PlatformBuild.py
   ```

4. **Run Stuart Update:**
   ```bash
   stuart_update -c Platforms/QemuQ35Pkg/PlatformBuild.py
   ```
   Note: Retry if you encounter "Filename too long" errors.

5. **Build and Launch:**
   ```bash
   # Windows
   stuart_build -c Platforms/QemuQ35Pkg/PlatformBuild.py --flashrom

   # Linux/WSL
   stuart_build -c Platforms/QemuQ35Pkg/PlatformBuild.py TOOL_CHAIN_TAG=CLANGPDB --flashrom
   ```

#### What PatinaSmbiosDxe Does

The driver performs the following operations:
1. Initializes a Patina component storage area
2. Provides boot services access to the Patina component
3. Creates an SMBIOS provider component with version 3.9
4. Initializes and runs the SMBIOS component, which:
   - Installs the SMBIOS protocol (GUID: 03583FF6-CB36-4940-947E-B9B39F4AFAF7)
   - Publishes the initial SMBIOS table
   - Integrates QEMU's SMBIOS table data into the UEFI environment

#### Verifying SMBIOS Integration

When QEMU launches, check the debug log for these messages:
- `PatinaSmbiosDxe Entry` - Driver has started
- `SMBIOS component initialized, protocol installed, and table published successfully` - Successful initialization
- Look for `InstallProtocolInterface: 03583FF6-CB36-4940-947E-B9B39F4AFAF7` from the DXE core

You can also verify SMBIOS data is available by using the `smbiosview` command in the UEFI shell.

#### Cargo Workspace Integration

The PatinaSmbiosDxe driver is part of the Cargo workspace defined in the root `Cargo.toml`. The workspace includes:
```toml
members = [
    "Platforms/PATINA_TIANOCORE_DEV/PatinaSmbiosDxe",
    # ... other members
]
```

This allows for unified dependency management and enables running `cargo build` or `cargo test` from the repository root to build all Rust components together.

#### Updating an Existing Checkout

If you have an existing checkout and need to update to the latest changes:

```bash
# Pull the latest changes
git pull

# Sync submodule URLs and branches to match .gitmodules
git submodule sync

# Update all submodules to the commits specified in the repository
git submodule update --init --recursive

# Alternatively, update to the latest commit on tracked branches
git submodule update --init --recursive --remote
```

**If you have local changes in submodules:**
```bash
# Save local changes in the submodule (e.g., Common/MU)
cd Common/MU
git stash
cd ../..

# Update submodules
git submodule update --init --recursive

# Optionally restore changes
cd Common/MU
git stash pop
cd ../..
```

After updating submodules, re-run the build steps starting from step 3 (Stuart Setup).

Note that this method of using the patina-smbios driver does not follow the spirit intended by the Patina project, so this repository will never go public and is only intended as an example of how to transition from the Tiano build environment to the Patina environment in small incremental steps.
