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

// REMOVED DUE TO EDITOR BEHAVIOR  -  #[cfg(target_os = "uefi")]


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

        // Setup a global allocator that uses the boot services memory functions for any normal crate
        // used outside the component we are supporting
        rust_boot_services_allocator_dxe::GLOBAL_ALLOCATOR.init(
            unsafe {
                (*system_table).boot_services 
            }
        );

        // Setup logging for this driver.  The advanced logger is currently being used so that we can
        // use the debug flags such as `DEBUG_INFO`, but any logging crate that supports no_std can be used.
        init_debug(unsafe { (*system_table).boot_services });
        debugln!(DEBUG_INFO, "PatinaSmbiosDxe Entry");

        // Create a Patina component storage area
        let mut storage = Storage::new();

        // Provide boot services access to the storage area
        storage.set_boot_services(
            StandardBootServices::new(
                unsafe { system_table.as_ref().unwrap().boot_services.as_ref().unwrap() }
            )
        );

        // Add the SMBIOS config to the storage area.
        // Default is version 3.9, so this is not necessary and only present for demonstration purposes.
        storage.add_config(
            SmbiosConfiguration { major_version: 3, minor_version: 9 }
        );

        // Create the SMBIOS component
        let mut smbios_component = SmbiosProviderManager::new().into_component();

        // Initialize the new component using the storage that has boot services and it's config
        smbios_component.initialize(&mut storage);

        // Run the component and convert it's return to an EFI_STATUS code
        match smbios_component.run(&mut storage) {
            Ok(_) => {
                // In the log, the line prior to this should state:
                // INFO - InstallProtocolInterface: 03583FF6-CB36-4940-947E-B9B39F4AFAF7
                // This is from the Tiano DXE core indicating the SMBIOS protocol was installed
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
