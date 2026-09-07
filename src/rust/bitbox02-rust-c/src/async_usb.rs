// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::missing_safety_doc)]

extern crate alloc;

use crate::HalImpl;
use alloc::vec::Vec;
use bitbox02_rust::async_usb::{on_next_request, spawn, waiting_for_next_request};

async fn process_packet_with_hal(usb_in: Vec<u8>) -> Vec<u8> {
    let mut hal = HalImpl::new();
    bitbox02_rust::hww::process_packet(&mut hal, usb_in).await
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_async_usb_spin() {
    bitbox02_rust::async_usb::spin();
}

#[repr(C)]
pub enum UsbResponse {
    UsbResponseAck,
    UsbResponseNotReady,
    UsbResponseNack,
}

/// Polls for a result of an async usb task. If a result is available, it is copied to `out`.
///
/// Returns:
/// `UsbResponseNack` if no task is running or the response does not fit in `out`.
/// An oversized response cancels the task, including any pending continuation.
/// `UsbResponseAck` if the result was copied.
/// `UsbResponseNotReady` if a task is running but not yet complete.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_async_usb_copy_response(out: *mut bitbox02::buffer_t) -> UsbResponse {
    use bitbox02_rust::async_usb::{CopyResponseErr, take_response};
    let dst = unsafe { core::slice::from_raw_parts_mut((*out).data, (*out).max_len) };
    match take_response() {
        Ok(response) => {
            let len = response.len();
            if len > dst.len() {
                // This also drops a workflow waiting for the next request after an intermediate
                // response that cannot be delivered. The host can start a fresh request/session.
                bitbox02_rust::async_usb::cancel();
                unsafe { (*out).len = 0 };
                return UsbResponse::UsbResponseNack;
            }
            dst[..len].copy_from_slice(&response);
            unsafe { (*out).len = len as _ };
            UsbResponse::UsbResponseAck
        }
        Err(CopyResponseErr::NotReady) => UsbResponse::UsbResponseNotReady,
        Err(CopyResponseErr::NotRunning) => UsbResponse::UsbResponseNack,
    }
}

/// Spawns the async HWW api processor (api level, HWW_REQ_*
/// arbitration level should be taken care of before).
///
/// `usb_in` are the api request bytes.
#[unsafe(no_mangle)]
pub extern "C" fn rust_async_usb_on_request_hww(usb_in: util::bytes::Bytes) {
    if waiting_for_next_request() {
        on_next_request(usb_in.as_ref());
    } else {
        spawn(process_packet_with_hal, usb_in.as_ref());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_async_usb_cancel() {
    bitbox02_rust::async_usb::cancel()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitbox02_rust::async_usb::{cancel, is_idle, next_request, spin};

    #[test]
    fn test_rust_async_usb_copy_response() {
        cancel();
        async fn task(response: Vec<u8>) -> Vec<u8> {
            response
        }
        let mut data = [0xaa; 4];
        let mut out = bitbox02::buffer_t {
            data: data.as_mut_ptr(),
            len: 0,
            max_len: data.len(),
        };
        assert!(matches!(
            unsafe { rust_async_usb_copy_response(&mut out) },
            UsbResponse::UsbResponseNack,
        ));

        // Oversized, exact-fit, short, and empty responses, with recovery after rejection.
        for response in [&[1, 2, 3, 4, 5][..], &[1, 2, 3, 4], &[1, 2], &[]] {
            data.fill(0xaa);
            spawn(task, response);
            assert!(matches!(
                unsafe { rust_async_usb_copy_response(&mut out) },
                UsbResponse::UsbResponseNotReady,
            ));
            spin();
            let result = unsafe { rust_async_usb_copy_response(&mut out) };
            if response.len() > data.len() {
                assert!(matches!(result, UsbResponse::UsbResponseNack));
                assert_eq!(out.len, 0);
                assert_eq!(data, [0xaa; 4]);
            } else {
                assert!(matches!(result, UsbResponse::UsbResponseAck));
                assert_eq!(out.len, response.len());
                assert_eq!(&data[..out.len], response);
                assert!(data[out.len..].iter().all(|byte| *byte == 0xaa));
            }
            assert!(is_idle());
        }

        // A rejected intermediate response must not leave its workflow waiting for input.
        async fn intermediate(response: Vec<u8>) -> Vec<u8> {
            next_request(response).await;
            panic!("oversized intermediate response must cancel the workflow");
        }
        spawn(intermediate, &[1, 2, 3, 4, 5]);
        spin();
        assert!(matches!(
            unsafe { rust_async_usb_copy_response(&mut out) },
            UsbResponse::UsbResponseNack,
        ));
        assert!(is_idle());
        assert!(!waiting_for_next_request());

        spawn(task, &[1]);
        spin();
        assert!(matches!(
            unsafe { rust_async_usb_copy_response(&mut out) },
            UsbResponse::UsbResponseAck,
        ));
        assert_eq!(out.len, 1);
        assert_eq!(data[0], 1);
        assert!(is_idle());
    }
}
