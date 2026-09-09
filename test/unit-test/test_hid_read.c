// SPDX-License-Identifier: Apache-2.0

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <cmocka.h>

#include <hpl_usb.h>
#include <usb_size.h>

#include <usb/class/hid/hww/hid_hww.h>
#include <usb/class/hid/u2f/hid_u2f.h>

#include <string.h>

typedef uint8_t (*rx_callback_t)(uint8_t, enum usb_xfer_code, uint32_t);

static FUNC_PTR _rx_callback = NULL;
static uint8_t* _pending_buf = NULL;
static uint32_t _pending_size = 0;
static size_t _read_calls = 0;

int32_t __wrap_hid_read(struct hid_func_data* func_data, uint8_t* buf, uint32_t size)
{
    (void)func_data;
    _pending_buf = buf;
    _pending_size = size;
    _read_calls++;
    return ERR_NONE;
}

int32_t __wrap_hid_register_callback(
    struct hid_func_data* func_data,
    enum hid_trans_type trans_type,
    FUNC_PTR func)
{
    (void)func_data;
    if (trans_type == HID_CB_READ) {
        _rx_callback = func;
    }
    return ERR_NONE;
}

int32_t __wrap_hid_req(
    struct usbdf_driver* drv,
    uint8_t ep,
    struct usb_req* req,
    enum usb_ctrl_stage stage)
{
    (void)drv;
    (void)ep;
    (void)req;
    (void)stage;
    return ERR_NONE;
}

static void _test_read_uses_owned_buffer(
    void (*setup)(void),
    bool (*read)(uint8_t* data))
{
    _rx_callback = NULL;
    _pending_buf = NULL;
    _pending_size = 0;
    _read_calls = 0;

    setup();
    assert_non_null(_rx_callback);

    uint8_t first_caller[USB_HID_REPORT_OUT_SIZE];
    uint8_t untouched[USB_HID_REPORT_OUT_SIZE];
    memset(first_caller, 0xa5, sizeof(first_caller));
    memcpy(untouched, first_caller, sizeof(untouched));

    assert_false(read(first_caller));
    assert_non_null(_pending_buf);
    assert_true(_pending_buf != first_caller);
    assert_int_equal((uintptr_t)_pending_buf % 4, 0);
    assert_int_equal(_pending_size, sizeof(first_caller));
    assert_int_equal(_read_calls, 1);

    uint8_t report[USB_HID_REPORT_OUT_SIZE];
    for (size_t i = 0; i < sizeof(report); i++) {
        report[i] = (uint8_t)(i + 1);
    }
    memcpy(_pending_buf, report, sizeof(report));
    assert_memory_equal(first_caller, untouched, sizeof(first_caller));

    assert_false(read(first_caller));
    assert_memory_equal(first_caller, untouched, sizeof(first_caller));
    assert_int_equal(_read_calls, 1);

    ((rx_callback_t)_rx_callback)(0, USB_XFER_DONE, sizeof(report));

    uint8_t completed_caller[USB_HID_REPORT_OUT_SIZE];
    memset(completed_caller, 0x5a, sizeof(completed_caller));
    assert_true(read(completed_caller));
    assert_memory_equal(completed_caller, report, sizeof(completed_caller));
    assert_memory_equal(first_caller, untouched, sizeof(first_caller));

    memset(_pending_buf, 0x3c, sizeof(report));
    assert_memory_equal(completed_caller, report, sizeof(completed_caller));

    uint8_t next_caller[USB_HID_REPORT_OUT_SIZE];
    memset(next_caller, 0x69, sizeof(next_caller));
    uint8_t next_untouched[USB_HID_REPORT_OUT_SIZE];
    memcpy(next_untouched, next_caller, sizeof(next_untouched));
    assert_false(read(next_caller));
    assert_memory_equal(next_caller, next_untouched, sizeof(next_caller));
    assert_int_equal(_read_calls, 2);
}

static void test_hid_hww_read_uses_owned_buffer(void** state)
{
    (void)state;
    _test_read_uses_owned_buffer(hid_hww_setup, hid_hww_read);
}

static void test_hid_u2f_read_uses_owned_buffer(void** state)
{
    (void)state;
    _test_read_uses_owned_buffer(hid_u2f_setup, hid_u2f_read);
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(test_hid_hww_read_uses_owned_buffer),
        cmocka_unit_test(test_hid_u2f_read_uses_owned_buffer),
    };
    return cmocka_run_group_tests(tests, NULL, NULL);
}
