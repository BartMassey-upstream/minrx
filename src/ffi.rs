//! FFI (Foreign Function Interface) layer
//!
//! Provides C-compatible API matching the original minrx.h interface.

use crate::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct minrx_regex_t {
    re_regexp: *mut Regex,
    re_nsub: libc::size_t,
    re_compflags: u32,
}

#[repr(C)]
pub struct minrx_regmatch_t {
    pub rm_so: libc::ptrdiff_t,
    pub rm_eo: libc::ptrdiff_t,
}

/// Compiles a regular expression pattern.
///
/// # Safety
///
/// `rx` must be a valid pointer to uninitialized `minrx_regex_t` struct.
/// `pattern` must be a valid null-terminated C string pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn minrx_regcomp(
    rx: *mut minrx_regex_t,
    pattern: *const c_char,
    flags: c_int,
) -> c_int {
    if rx.is_null() || pattern.is_null() {
        return RegexError::BadPat as c_int;
    }

    let pattern_str = unsafe {
        match CStr::from_ptr(pattern).to_str() {
            Ok(s) => s,
            Err(_) => return RegexError::BadPat as c_int,
        }
    };

    let compile_flags = CompileFlags::from_bits_truncate(flags as u32);

    match Regex::new(pattern_str, compile_flags) {
        Ok(regex) => {
            let nsub = regex.nsub();
            let boxed = Box::new(regex);
            unsafe {
                (*rx).re_regexp = Box::into_raw(boxed);
                (*rx).re_nsub = nsub;
                (*rx).re_compflags = flags as u32;
            }
            RegexError::Success as c_int
        }
        Err(e) => e as c_int,
    }
}

/// Executes a compiled regular expression against input text.
///
/// # Safety
///
/// `rx` must be a valid pointer to a `minrx_regex_t` initialized by `minrx_regcomp`.
/// `text` must be a valid null-terminated C string pointer.
/// If `pmatch` is not null, it must point to an array of at least `nmatch` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn minrx_regexec(
    rx: *const minrx_regex_t,
    text: *const c_char,
    nmatch: libc::size_t,
    pmatch: *mut minrx_regmatch_t,
    eflags: c_int,
) -> c_int {
    if rx.is_null() || text.is_null() {
        return RegexError::BadPat as c_int;
    }

    let regex = unsafe {
        let regex_ptr = (*rx).re_regexp;
        if regex_ptr.is_null() {
            return RegexError::BadPat as c_int;
        }
        &*regex_ptr
    };

    let text_str = unsafe {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return RegexError::BadPat as c_int,
        }
    };

    let exec_flags = ExecFlags::from_bits_truncate(eflags as u32);

    match regex.exec(text_str, exec_flags) {
        Ok(matches) => {
            if !pmatch.is_null() {
                for (i, m) in matches.iter().enumerate().take(nmatch) {
                    unsafe {
                        match m {
                            Some(range) => {
                                (*pmatch.add(i)).rm_so = range.start as libc::ptrdiff_t;
                                (*pmatch.add(i)).rm_eo = range.end as libc::ptrdiff_t;
                            }
                            None => {
                                (*pmatch.add(i)).rm_so = -1;
                                (*pmatch.add(i)).rm_eo = -1;
                            }
                        }
                    }
                }
            }
            RegexError::Success as c_int
        }
        Err(e) => e as c_int,
    }
}

/// Frees resources associated with a compiled regular expression.
///
/// # Safety
///
/// `rx` must be a valid pointer to a `minrx_regex_t` initialized by `minrx_regcomp`.
/// After calling this function, `rx` must not be used with `minrx_regexec`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn minrx_regfree(rx: *mut minrx_regex_t) {
    if !rx.is_null() {
        unsafe {
            let regex_ptr = (*rx).re_regexp;
            if !regex_ptr.is_null() {
                let _ = Box::from_raw(regex_ptr);
                (*rx).re_regexp = ptr::null_mut();
            }
        }
    }
}

/// Returns an error message string for a given error code.
///
/// # Safety
///
/// If `errbuf` is not null, it must point to a buffer of at least `errbuf_size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn minrx_regerror(
    errcode: c_int,
    _rx: *const minrx_regex_t,
    errbuf: *mut c_char,
    errbuf_size: libc::size_t,
) -> libc::size_t {
    let error = match errcode {
        0 => RegexError::Success,
        1 => RegexError::BadPat,
        2 => RegexError::BadBr,
        3 => RegexError::BadRpt,
        4 => RegexError::EBrace,
        5 => RegexError::EBrack,
        6 => RegexError::ECollate,
        7 => RegexError::ECType,
        8 => RegexError::EEscape,
        9 => RegexError::EParen,
        10 => RegexError::ERange,
        11 => RegexError::ESpace,
        12 => RegexError::ESubreg,
        13 => RegexError::NoMatch,
        _ => RegexError::Unknown,
    };

    let message = error.message();
    let message_bytes = message.as_bytes();
    let len = message_bytes.len();

    if !errbuf.is_null() && errbuf_size > 0 {
        unsafe {
            let copy_len = (len + 1).min(errbuf_size);
            ptr::copy_nonoverlapping(
                message_bytes.as_ptr() as *const c_char,
                errbuf,
                copy_len - 1,
            );
            *errbuf.add(copy_len - 1) = 0;
        }
    }

    len + 1
}

/// Compiles a regular expression pattern with explicit length.
///
/// # Safety
///
/// `rx` must be a valid pointer to uninitialized `minrx_regex_t` struct.
/// `pattern` must be a valid pointer to a buffer of at least `npattern` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn minrx_regncomp(
    rx: *mut minrx_regex_t,
    npattern: libc::size_t,
    pattern: *const c_char,
    flags: c_int,
) -> c_int {
    if rx.is_null() || pattern.is_null() {
        return RegexError::BadPat as c_int;
    }

    let pattern_slice = unsafe { std::slice::from_raw_parts(pattern as *const u8, npattern) };

    let pattern_str = match std::str::from_utf8(pattern_slice) {
        Ok(s) => s,
        Err(_) => return RegexError::BadPat as c_int,
    };

    let compile_flags = CompileFlags::from_bits_truncate(flags as u32);

    match Regex::new(pattern_str, compile_flags) {
        Ok(regex) => {
            let nsub = regex.nsub();
            let boxed = Box::new(regex);
            unsafe {
                (*rx).re_regexp = Box::into_raw(boxed);
                (*rx).re_nsub = nsub;
                (*rx).re_compflags = flags as u32;
            }
            RegexError::Success as c_int
        }
        Err(e) => e as c_int,
    }
}

/// Executes a compiled regular expression against input text with explicit length.
///
/// # Safety
///
/// `rx` must be a valid pointer to a `minrx_regex_t` initialized by `minrx_regcomp` or `minrx_regncomp`.
/// `text` must be a valid pointer to a buffer of at least `ntext` bytes.
/// If `pmatch` is not null, it must point to an array of at least `nmatch` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn minrx_regnexec(
    rx: *const minrx_regex_t,
    ntext: libc::size_t,
    text: *const c_char,
    nmatch: libc::size_t,
    pmatch: *mut minrx_regmatch_t,
    eflags: c_int,
) -> c_int {
    if rx.is_null() || text.is_null() {
        return RegexError::BadPat as c_int;
    }

    let regex = unsafe {
        let regex_ptr = (*rx).re_regexp;
        if regex_ptr.is_null() {
            return RegexError::BadPat as c_int;
        }
        &*regex_ptr
    };

    let text_slice = unsafe { std::slice::from_raw_parts(text as *const u8, ntext) };

    let text_str = match std::str::from_utf8(text_slice) {
        Ok(s) => s,
        Err(_) => return RegexError::BadPat as c_int,
    };

    let exec_flags = ExecFlags::from_bits_truncate(eflags as u32);

    match regex.exec(text_str, exec_flags) {
        Ok(matches) => {
            if !pmatch.is_null() {
                for (i, m) in matches.iter().enumerate().take(nmatch) {
                    unsafe {
                        match m {
                            Some(range) => {
                                (*pmatch.add(i)).rm_so = range.start as libc::ptrdiff_t;
                                (*pmatch.add(i)).rm_eo = range.end as libc::ptrdiff_t;
                            }
                            None => {
                                (*pmatch.add(i)).rm_so = -1;
                                (*pmatch.add(i)).rm_eo = -1;
                            }
                        }
                    }
                }
            }
            RegexError::Success as c_int
        }
        Err(e) => e as c_int,
    }
}
