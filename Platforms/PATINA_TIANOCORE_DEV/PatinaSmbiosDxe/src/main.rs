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
    use patina::{
        boot_services::StandardBootServices,
        component::{IntoComponent, Storage},
    };
    use patina_smbios::component::{SmbiosConfiguration, SmbiosProviderManager};
    use r_efi::efi::Status;
    use rust_advanced_logger_dxe::{debugln, init_debug, DEBUG_ERROR, DEBUG_INFO};

    #[no_mangle]
    pub extern "efiapi" fn efi_main(
        _image_handle: *const core::ffi::c_void,
        system_table: *const r_efi::system::SystemTable,
    ) -> u64 {
        //
        // Normal driver setup to allow a debugger
        //

        rust_boot_services_allocator_dxe::GLOBAL_ALLOCATOR.init(unsafe { (*system_table).boot_services });
        init_debug(unsafe { (*system_table).boot_services });
        debugln!(DEBUG_INFO, "PatinaSmbiosDxe Entry");

        //
        // SMBIOS Module Usage
        //

        // Step 1: Create Patina component storage
        let mut storage = Storage::new();

        // Step 2: Set up boot services in storage
        let system_table_ref = unsafe { &*system_table };
        let boot_services_ref = unsafe { &*system_table_ref.boot_services };
        let boot_services = StandardBootServices::new(boot_services_ref);
        storage.set_boot_services(boot_services.clone());

        // Step 3: Configure SMBIOS (default is version 3.9, defining here as example)
        let config = SmbiosConfiguration { major_version: 3, minor_version: 9 };
        storage.add_config(config);

        // Step 4: Create the SMBIOS provider
        let smbios_provider = SmbiosProviderManager::new();

        // Step 5: Create and initialize the SMBIOS component
        let mut smbios_component = smbios_provider.into_component();
        smbios_component.initialize(&mut storage);

        // Step 6: Run the component and convert it's return to an EFI_STATUS code
        match smbios_component.run(&mut storage) {
            Ok(_) => {
                debugln!(DEBUG_INFO, "SMBIOS component run completed successfully");
                Status::SUCCESS.as_usize() as u64
            }
            Err(e) => {
                debugln!(DEBUG_ERROR, "SMBIOS component run failed with error: {:?}", e);
                Status::LOAD_ERROR.as_usize() as u64
            }
        }
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
