// Deny some rustc lints that are allow-by-default.
#![deny(
    ambiguous_negative_literals,
    impl_trait_overcaptures,
    let_underscore_drop,
    missing_copy_implementations,
    missing_debug_implementations,
    non_ascii_idents,
    redundant_imports,
    redundant_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unit_bindings,
    unreachable_pub
)]
#![deny(clippy::pedantic)]
// Allow a few clippy pedantic lints.
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_safety_doc)]
// Selectively deny clippy restriction lints.
#![deny(
    clippy::as_conversions,
    clippy::as_underscore,
    clippy::assertions_on_result_states,
    clippy::big_endian_bytes,
    clippy::cfg_not_test,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::default_numeric_fallback,
    clippy::doc_include_without_cfg,
    clippy::empty_drop,
    clippy::error_impl_error,
    clippy::exit,
    clippy::exhaustive_enums,
    clippy::expect_used,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast_any,
    clippy::get_unwrap,
    clippy::host_endian_bytes,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::infinite_loop,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::iter_over_hash_type,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::map_err_ignore,
    clippy::map_with_unused_argument_over_ranges,
    clippy::mem_forget,
    clippy::missing_assert_message,
    clippy::missing_asserts_for_indexing,
    clippy::mixed_read_write_in_expression,
    clippy::multiple_unsafe_ops_per_block,
    clippy::mutex_atomic,
    clippy::mutex_integer,
    clippy::needless_raw_strings,
    clippy::non_ascii_literal,
    clippy::non_zero_suggestions,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::partial_pub_fields,
    clippy::pathbuf_init_then_push,
    clippy::precedence_bits,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::redundant_type_annotations,
    clippy::ref_patterns,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::str_to_string,
    clippy::string_lit_chars_any,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suspicious_xor_used_as_pow,
    clippy::tests_outside_test_module,
    clippy::todo,
    clippy::try_err,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unnecessary_safety_comment,
    clippy::unneeded_field_pattern,
    clippy::unreachable,
    clippy::unused_result_ok,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::verbose_file_reads
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
