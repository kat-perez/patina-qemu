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
    use rust_advanced_logger_dxe::{debugln, init_debug, DEBUG_ERROR};
    use patina_smbios;

    #[no_mangle]
    pub extern "efiapi" fn efi_main(
        _image_handle: *const core::ffi::c_void,
        _system_table: *const r_efi::system::SystemTable,
    ) -> u64 {
        rust_boot_services_allocator_dxe::GLOBAL_ALLOCATOR.init(unsafe { (*_system_table).boot_services });
        init_debug(unsafe { (*_system_table).boot_services });

        let return_status = Status::SUCCESS.as_usize() as u64;

        debugln!(DEBUG_ERROR, "PatinaSmbiosDxe Entry");


        let sample_add = patina_smbios::add(2, 3);
        debugln!(DEBUG_ERROR, "patina_smbios::add(2, 3) = {}", sample_add);


        debugln!(DEBUG_ERROR, "PatinaSmbiosDxe Exit: {:?}", return_status);
        return_status
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

#[cfg(test)]
mod test {

    #[test]
    fn sample_test() {
        assert_eq!(1 + 1, 2);
    }
}
