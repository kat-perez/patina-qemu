//! SMBIOS DXE Driver
//!
//! Demonstrates wrapping a Patina SMBIOS module to create a Tianocore based EFI driver
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation. All rights reserved.
//!
//! SPDX-License-Identifier: MIT License
//!

#![no_std]
#![allow(non_snake_case)]
// UEFI requires "no_main" and a panic handler. To facilitate unit tests, gate "no_main", UEFI entry point, and panic
// handler behind target_os configuration.
#![cfg_attr(target_os = "uefi", no_main)]
#[cfg(target_os = "uefi")]

mod uefi_entry {
    extern crate alloc;

    use core::panic::PanicInfo;
    use r_efi::efi::Status;
    use rust_advanced_logger_dxe;
    use patina_smbios;
    use patina::component::IntoComponent;

    #[no_mangle]
    pub extern "efiapi" fn efi_main(
        _image_handle: *const core::ffi::c_void,
        _system_table: *const r_efi::system::SystemTable,
    ) -> u64 {

        //
        // Normal driver setup to allow a debugger
        //

        rust_boot_services_allocator_dxe::GLOBAL_ALLOCATOR.init(unsafe { (*_system_table).boot_services });
        rust_advanced_logger_dxe::init_debug(unsafe { (*_system_table).boot_services });
        rust_advanced_logger_dxe::debugln!(rust_advanced_logger_dxe::DEBUG_ERROR, "PatinaSmbiosDxe Entry");


        //
        // SMBIOS Module Usage
        // 

        // Step 1: Create component storage
        let mut storage = patina::component::Storage::new();

        // Step 2: Configure SMBIOS (default is version 3.9, defining here as example)
        let config = patina_smbios::SmbiosConfiguration {
            major_version: 3,
            minor_version: 9,
        };
        storage.add_config(config);

        // Step 3: Create and initialize the SMBIOS provider component
        let smbios_provider = patina_smbios::component::SmbiosProviderManager::new();
        let mut component = smbios_provider.into_component();
        component.initialize(&mut storage);

        // Step 4: Run the component - this calls entry_point() in the module
        let run_status = component.run(&mut storage);
        rust_advanced_logger_dxe::debugln!(rust_advanced_logger_dxe::DEBUG_ERROR, "smbios_component.run return {:?}", run_status);


        //
        // Normal driver exit.  Any Tianocore driver after this should be able to use the SMBIOS protocol.
        //

        rust_advanced_logger_dxe::debugln!(rust_advanced_logger_dxe::DEBUG_ERROR, "PatinaSmbiosDxe Exit");
        Status::SUCCESS.as_usize() as u64
    }

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}

// For non-UEFI targets (e.g. compiling for unit test or clippy), supply a "main" function.
#[cfg(not(target_os = "uefi"))]
fn main() {
    //do nothing.
}

// Test targets
#[cfg(test)]
mod test {

    #[test]
    fn sample_test() {
        assert_eq!(1 + 1, 2);
    }
}
