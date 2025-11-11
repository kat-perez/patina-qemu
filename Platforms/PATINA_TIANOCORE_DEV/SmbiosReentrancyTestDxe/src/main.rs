//! SMBIOS RefCell Reentrancy Test Driver
//!
//! This driver stresses the SMBIOS protocol with timer interrupts at TPL_NOTIFY
//! to detect potential RefCell reentrancy issues.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation. All rights reserved.
//!
//! SPDX-License-Identifier: MIT License
//!

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use r_efi::efi;

// SMBIOS Protocol GUID
const SMBIOS_PROTOCOL_GUID: efi::Guid = efi::Guid::from_fields(
    0x03583ff6,
    0xcb36,
    0x4940,
    0x94,
    0x7e,
    &[0xb9, 0xb3, 0x9f, 0x4a, 0xfa, 0xf7],
);

// SMBIOS Protocol function pointers (C ABI)
#[repr(C)]
struct SmbiosProtocol {
    add: extern "efiapi" fn(
        *const SmbiosProtocol,
        efi::Handle,
        *mut u16,
        *mut u8,
    ) -> efi::Status,
    update_string: extern "efiapi" fn(
        *const SmbiosProtocol,
        *mut u16,
        *mut usize,
        *const u8,
    ) -> efi::Status,
    remove: extern "efiapi" fn(*const SmbiosProtocol, u16) -> efi::Status,
    get_next: extern "efiapi" fn(
        *const SmbiosProtocol,
        *mut u16,
        *mut u8,
        *mut *mut u8,
        *mut efi::Handle,
    ) -> efi::Status,
    major_version: u8,
    minor_version: u8,
}

static mut TIMER_FIRE_COUNT: usize = 0;

/// Timer callback that calls SMBIOS GetNext() at TPL_NOTIFY
extern "efiapi" fn timer_callback(_event: efi::Event, context: *mut core::ffi::c_void) {
    if context.is_null() {
        return;
    }

    unsafe {
        TIMER_FIRE_COUNT += 1;

        let protocol = context as *const SmbiosProtocol;
        let mut handle: u16 = 0xFFFE; // Start from beginning
        let mut record_type: u8 = 0;
        let mut record_ptr: *mut u8 = core::ptr::null_mut();
        let mut producer_handle: efi::Handle = core::ptr::null_mut();

        // Call GetNext() - this will try_borrow() the manager
        let _ = ((*protocol).get_next)(
            protocol,
            &mut handle,
            &mut record_type,
            &mut record_ptr,
            &mut producer_handle,
        );
    }
}

#[no_mangle]
pub extern "efiapi" fn efi_main(
    _image_handle: efi::Handle,
    system_table: *const efi::SystemTable,
) -> efi::Status {
    unsafe {
        // Setup allocator
        rust_boot_services_allocator_dxe::GLOBAL_ALLOCATOR
            .init((*system_table).boot_services);

        // Setup logging
        rust_advanced_logger_dxe::init_debug((*system_table).boot_services);
    }

    log::info!("==================================================");
    log::info!("=== SMBIOS RefCell Reentrancy Stress Test ===");
    log::info!("==================================================");
    log::info!("This test creates a timer at TPL_NOTIFY that repeatedly");
    log::info!("calls SMBIOS GetNext() while adding records at TPL_APPLICATION.");
    log::info!("Watch for '⚠️  SMBIOS REENTRANCY DETECTED' messages!");
    log::info!("");

    unsafe {
        let boot_services = (*system_table).boot_services.as_ref().unwrap();

        // Step 1: Locate SMBIOS protocol
        let mut protocol_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let mut status = (boot_services.locate_protocol)(
            &SMBIOS_PROTOCOL_GUID as *const _ as *mut _,
            core::ptr::null_mut(),
            &mut protocol_ptr,
        );

        if status != efi::Status::SUCCESS {
            log::error!("Failed to locate SMBIOS protocol: {:?}", status);
            return status;
        }

        let protocol = protocol_ptr as *const SmbiosProtocol;
        log::info!("✓ SMBIOS protocol located at {:p}", protocol);
        log::info!("  Version: {}.{}", (*protocol).major_version, (*protocol).minor_version);

        // Step 2: Create timer event at TPL_NOTIFY
        let mut timer_event: efi::Event = core::ptr::null_mut();
        status = (boot_services.create_event)(
            efi::EVT_TIMER | efi::EVT_NOTIFY_SIGNAL,
            efi::TPL_NOTIFY,
            Some(timer_callback),
            protocol as *mut _,
            &mut timer_event,
        );

        if status != efi::Status::SUCCESS {
            log::error!("Failed to create timer event: {:?}", status);
            return status;
        }

        log::info!("✓ Timer event created at TPL_NOTIFY");

        // Step 3: Set timer to fire every 1ms
        const TIMER_RELATIVE: u32 = 1;
        status = (boot_services.set_timer)(
            timer_event,
            TIMER_RELATIVE,
            10_000, // 1ms in 100ns units
        );

        if status != efi::Status::SUCCESS {
            log::error!("Failed to set timer: {:?}", status);
            (boot_services.close_event)(timer_event);
            return status;
        }

        log::info!("✓ Timer set to fire every 1ms");
        log::info!("");
        log::info!("Starting stress test: Adding 200 SMBIOS records...");

        // Step 4: Add many records while timer fires
        for i in 0..200 {
            // Create a minimal Type 1 (System Information) record
            let mut record = alloc::vec![
                1u8,   // Type
                27,    // Length
                0xFF, 0xFE, // Handle (auto-assigned)
                1, 2, 3, 4, // String indices
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // UUID
                1,     // Wakeup type
                5, 6,  // More string indices
                b'M', b'f', b'g', 0, // Strings
                b'P', b'r', b'd', 0,
                b'V', b'e', b'r', 0,
                b'S', b'N', 0,
                b'S', b'K', b'U', 0,
                b'F', b'a', b'm', 0,
                0, // Double null terminator
            ];

            let mut handle: u16 = 0;
            status = ((*protocol).add)(
                protocol,
                core::ptr::null_mut(),
                &mut handle,
                record.as_mut_ptr(),
            );

            if status != efi::Status::SUCCESS {
                log::warn!("Add() at iteration {} returned: {:?}", i, status);
            }

            // Yield periodically to let timer fire
            if i % 10 == 0 {
                (boot_services.stall)(1000); // 1ms
            }
        }

        log::info!("✓ Finished adding 200 records");
        log::info!("  Timer fired {} times during test", TIMER_FIRE_COUNT);

        // Let timer fire a few more times
        log::info!("Waiting 50ms for additional timer fires...");
        (boot_services.stall)(50_000);
        log::info!("  Timer fired {} times total", TIMER_FIRE_COUNT);

        // Cleanup
        const TIMER_CANCEL: u32 = 0;
        (boot_services.set_timer)(timer_event, TIMER_CANCEL, 0);
        (boot_services.close_event)(timer_event);

        log::info!("");
        log::info!("==================================================");
        log::info!("=== Reentrancy Stress Test COMPLETE ===");
        log::info!("==================================================");
        log::info!("");
        log::info!("RESULTS:");
        if TIMER_FIRE_COUNT > 0 {
            log::info!("✓ Timer successfully fired {} times at TPL_NOTIFY", TIMER_FIRE_COUNT);
            log::info!("✓ Test successfully stressed the SMBIOS protocol");
            log::info!("");
            log::info!("If you saw '⚠️  SMBIOS REENTRANCY DETECTED' messages:");
            log::info!("  → RefCell is UNSAFE for SMBIOS manager");
            log::info!("  → Must use TplMutex or other synchronization");
            log::info!("");
            log::info!("If NO reentrancy detected:");
            log::info!("  → Either RefCell is safe in this environment");
            log::info!("  → Or the race window was too small to trigger");
            log::info!("  → Recommend testing on real hardware as well");
        } else {
            log::warn!("⚠ Timer never fired - test may be invalid!");
            log::warn!("  This could indicate QEMU timer issues");
        }

        efi::Status::SUCCESS
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
