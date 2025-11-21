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

    use alloc::boxed::Box;
    use core::panic::PanicInfo;
    use patina::{
        base::UEFI_PAGE_MASK,
        boot_services::{BootServices, StandardBootServices, allocation::AllocType},
        component::{IntoComponent, Storage, service::{
            IntoService,
            memory::{AccessType, AllocationOptions, CachingType, MemoryError, MemoryManager, PageAllocation, PageAllocationStrategy},
        }},
    };
    use patina_smbios::component::SmbiosProvider;
    use r_efi::efi::Status;
    use rust_advanced_logger_dxe::init_debug;

    /// BootServices-based MemoryManager implementation for TianoCore integration
    #[derive(IntoService)]
    #[service(dyn MemoryManager)]
    struct BootServicesMemoryManager {
        boot_services: &'static StandardBootServices,
    }

    impl BootServicesMemoryManager {
        fn new(boot_services: &'static StandardBootServices) -> &'static Self {
            Box::leak(Box::new(Self { boot_services }))
        }
    }

    impl MemoryManager for BootServicesMemoryManager {
        fn allocate_pages(&self, page_count: usize, options: AllocationOptions) -> Result<PageAllocation, MemoryError> {
            let alignment = options.alignment();

            if !alignment.is_power_of_two() || alignment & UEFI_PAGE_MASK != 0 {
                return Err(MemoryError::InvalidAlignment);
            }

            let alloc_type = match options.strategy() {
                PageAllocationStrategy::Any => AllocType::AnyPage,
                PageAllocationStrategy::Address(addr) => {
                    if addr % alignment != 0 {
                        return Err(MemoryError::UnalignedAddress);
                    }
                    AllocType::Address(addr)
                }
                PageAllocationStrategy::MaxAddress(max) => AllocType::MaxAddress(max),
            };

            let address = self.boot_services
                .allocate_pages(alloc_type, options.memory_type(), page_count)
                .map_err(|_| MemoryError::NoAvailableMemory)?;

            // SAFETY: This instance was created via Box::leak in new(), so it has 'static lifetime
            let self_static: &'static Self = unsafe { &*(self as *const _) };

            unsafe {
                PageAllocation::new(address as usize, page_count, self_static)
                    .map_err(|_| MemoryError::InternalError)
            }
        }

        unsafe fn free_pages(&self, address: usize, page_count: usize) -> Result<(), MemoryError> {
            self.boot_services
                .free_pages(address, page_count)
                .map_err(|_| MemoryError::InvalidAddress)?;
            Ok(())
        }

        unsafe fn set_page_attributes(
            &self,
            _address: usize,
            _page_count: usize,
            _access: AccessType,
            _caching: Option<CachingType>,
        ) -> Result<(), MemoryError> {
            // TianoCore doesn't expose page attribute setting through boot services
            Err(MemoryError::InternalError)
        }

        fn get_page_attributes(
            &self,
            _address: usize,
            _page_count: usize,
        ) -> Result<(AccessType, CachingType), MemoryError> {
            // TianoCore doesn't expose page attribute querying through boot services
            Err(MemoryError::InternalError)
        }
    }

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
        // Create a Patina component storage area and leak it so it lives forever
        // This is required because TplMutex holds a 'static reference to boot_services from Storage
        let storage = Box::leak(Box::new(Storage::new()));
        log::info!("Storage created and leaked successfully");

        log::info!("Setting boot services...");
        // Provide boot services access to the storage area
        let boot_services_ref = StandardBootServices::new(
            unsafe { system_table.as_ref().unwrap().boot_services.as_ref().unwrap() }
        );
        storage.set_boot_services(boot_services_ref);

        // SAFETY: Storage guarantees boot_services has 'static lifetime
        // We need a 'static reference for the MemoryManager
        let boot_services_static: &'static StandardBootServices =
            unsafe { &*(storage.boot_services() as *const _) };
        log::info!("Boot services set successfully");

        log::info!("Creating and registering MemoryManager...");
        // Create and register the MemoryManager service
        let memory_manager = BootServicesMemoryManager::new(boot_services_static);
        storage.add_service(memory_manager);
        log::info!("MemoryManager registered successfully");

        log::info!("Creating SmbiosProvider with direct installation...");
        // Create the SMBIOS component with version 3.9
        // The protocol will be installed immediately during component initialization
        let smbios_provider = SmbiosProvider::new(3, 9);
        log::info!("SmbiosProvider created, converting to component...");
        let mut smbios_component = smbios_provider.into_component();
        log::info!("Component conversion complete");

        log::info!("Initializing SMBIOS component...");
        smbios_component.initialize(storage);
        log::info!("Component initialized, running component...");

        // Run the component, which initializes the manager and installs the protocol
        match smbios_component.run(storage) {
            Ok(_) => {
                log::info!("SMBIOS component initialized successfully");
                log::info!("Protocol installed and ready for use");
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
