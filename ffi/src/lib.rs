#![allow(
    unsafe_code,
    clippy::allow_attributes,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc
)]
#![cfg_attr(
    test,
    allow(
        clippy::assertions_on_result_states,
        clippy::indexing_slicing,
        clippy::missing_asserts_for_indexing,
        clippy::panic,
        clippy::unwrap_used,
    )
)]
mod constants;
mod helpers;
mod state;

use std::cell::RefCell;
use std::ffi::{c_char, c_int, CString};
use std::panic::catch_unwind;
use std::ptr;
use std::str::FromStr;

use loot_condition_interpreter::Expression;

pub use constants::*;
use helpers::{error, handle_error, to_str};
pub use state::*;

thread_local!(static ERROR_MESSAGE: RefCell<CString> = RefCell::default());

#[no_mangle]
pub unsafe extern "C" fn lci_condition_parse(condition: *const c_char) -> c_int {
    catch_unwind(|| {
        if condition.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null pointer passed")
        } else {
            let expression = match to_str(condition) {
                Ok(x) => x,
                Err(e) => return e,
            };

            if let Err(e) = Expression::from_str(expression) {
                handle_error(&e)
            } else {
                LCI_OK
            }
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_condition_eval(
    condition: *const c_char,
    state: *mut lci_state,
) -> c_int {
    catch_unwind(|| {
        if condition.is_null() || state.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null pointer passed")
        } else {
            let expression = match to_str(condition) {
                Ok(x) => x,
                Err(e) => return e,
            };

            let expression = match Expression::from_str(expression) {
                Err(e) => return handle_error(&e),
                Ok(x) => x,
            };

            let state = match (*state).0.read() {
                Err(e) => return error(LCI_ERROR_POISONED_THREAD_LOCK, &e.to_string()),
                Ok(s) => s,
            };

            match expression.eval(&state) {
                Ok(true) => LCI_RESULT_TRUE,
                Ok(false) => LCI_RESULT_FALSE,
                Err(e) => handle_error(&e),
            }
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}

#[no_mangle]
pub unsafe extern "C" fn lci_get_error_message(message: *mut *const c_char) -> c_int {
    catch_unwind(|| {
        if message.is_null() {
            error(LCI_ERROR_INVALID_ARGS, "Null pointer passed")
        } else {
            ERROR_MESSAGE.with(|f| {
                if f.borrow().as_bytes().is_empty() {
                    *message = ptr::null();
                } else {
                    *message = f.borrow().as_ptr();
                }
            });

            LCI_OK
        }
    })
    .unwrap_or(LCI_ERROR_PANICKED)
}
