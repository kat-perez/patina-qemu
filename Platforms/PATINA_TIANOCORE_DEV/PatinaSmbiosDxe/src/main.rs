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
    use patina_smbios::component::SmbiosProvider;
    use r_efi::efi::Status;
    use rust_advanced_logger_dxe::init_debug;

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

        // Setup logging for this driver using the log crate bridge
        unsafe {
            init_debug((*system_table).boot_services);
        }
        log::info!("PatinaSmbiosDxe Entry");

        log::info!("Creating Storage...");
        // Create a Patina component storage area
        let mut storage = Storage::new();
        log::info!("Storage created successfully");

        log::info!("Setting boot services...");
        // Provide boot services access to the storage area
        storage.set_boot_services(
            StandardBootServices::new(
                unsafe { system_table.as_ref().unwrap().boot_services.as_ref().unwrap() }
            )
        );
        log::info!("Boot services set successfully");

        log::info!("Creating SmbiosProvider...");
        // Create the SMBIOS component with version 3.9 and convert to component
        let smbios_provider = SmbiosProvider::new(3, 9);
        log::info!("SmbiosProvider created, converting to component...");
        let mut smbios_component = smbios_provider.into_component();
        log::info!("Component conversion complete");

        log::info!("Initializing SMBIOS component...");
        smbios_component.initialize(&mut storage);
        log::info!("Component initialized, running component...");

        // Run the component, which initializes the manager, installs the SMBIOS protocol,
        // and publishes the initial SMBIOS table
        match smbios_component.run(&mut storage) {
            Ok(_) => {
                // In the log, the line prior to this should state:
                // INFO - InstallProtocolInterface: 03583FF6-CB36-4940-947E-B9B39F4AFAF7
                // This is from the Tiano DXE core indicating the SMBIOS protocol was installed
                log::info!("SMBIOS component initialized, protocol installed, and table published successfully");
                Status::SUCCESS.as_usize() as u64
            }
            Err(e) => {
                log::error!("SMBIOS component failed with error: {:?}", e);
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
