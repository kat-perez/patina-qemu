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


## Repo 3: Private QEMU UEFI build for testing the Patina SMBIOS module as a Tianocore driver

[PRIVATE patina-tianocore-dev - Branch: main](https://github.com/rogurr/patina-tianocore-dev)

This repo contains a copy of the patina-qemu tree with the following changes:
* Disable the Patina DXE core
* Enable the Tiano DXE core
* Add a ```/Platforms/PATINA_TIANOCORE_DEV/PatinaSmbiosDxe``` driver

The new driver is written in Rust and demonstrates how to use and publish normal Tianocore interfaces while using a Patina component.  Build and test is done by running through the steps outlined in QemuReadme.md (the original readme.md file from this repo).

Note that this method of using the patina-smbios driver does not follow the spirit intended by the Patina project, so this repository will never go public and is only intended as an example of how to transition from the Tiano build environment to the Patina environment in small incremental steps.
