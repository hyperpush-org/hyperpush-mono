//! Regex support for the Mesh standard library.
//!
//! Provides mesh_regex_from_literal, mesh_regex_compile, mesh_regex_match,
//! mesh_regex_captures, mesh_regex_replace, and mesh_regex_split.
//!
//! Regex objects are heap-allocated via Box<regex::Regex> and returned as
//! opaque raw pointers. They are never freed (GC-managed programs).

use crate::collections::list::{mesh_list_builder_new, mesh_list_builder_push};
use crate::option::alloc_option;
use crate::string::{mesh_string_new, MeshString};
use regex::RegexBuilder;

// ── Internal helper ────────────────────────────────────────────────────

/// Build a `regex::Regex` with flag bits applied.
///
/// Flags bitmask: i=1, m=2, s=4  (same encoding used by mesh_regex_from_literal).
unsafe fn build_with_flags(pattern: &str, flags_bits: i64) -> Result<regex::Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(flags_bits & 1 != 0)
        .multi_line(flags_bits & 2 != 0)
        .dot_matches_new_line(flags_bits & 4 != 0)
        .build()
}

// ── Public ABI ─────────────────────────────────────────────────────────

/// Compile a regex literal at program start. Called by desugared ~r/pat/flags.
///
/// Panics on compile error (the pattern is a compile-time literal, so errors
/// should have been caught by the developer before shipping).
#[no_mangle]
pub extern "C" fn mesh_regex_from_literal(
    pattern: *const MeshString,
    flags_bits: i64,
) -> *mut u8 {
    unsafe {
        let pat = (*pattern).as_str();
        match build_with_flags(pat, flags_bits) {
            Ok(rx) => Box::into_raw(Box::new(rx)) as *mut u8,
            Err(e) => panic!("mesh_regex_from_literal: invalid regex pattern {:?}: {}", pat, e),
        }
    }
}

/// Regex.compile(pattern) -> Result<Regex, String>
///
/// Returns a MeshOption with tag=0 (Ok) containing the compiled regex pointer,
/// or tag=1 (Err) containing a MeshString with the error message.
#[no_mangle]
pub extern "C" fn mesh_regex_compile(pattern: *const MeshString) -> *mut u8 {
    unsafe {
        let pat = (*pattern).as_str();
        match regex::Regex::new(pat) {
            Ok(rx) => {
                let rx_ptr = Box::into_raw(Box::new(rx)) as *mut u8;
                alloc_option(0, rx_ptr) as *mut u8
            }
            Err(e) => {
                let msg = e.to_string();
                let err_str = mesh_string_new(msg.as_ptr(), msg.len() as u64);
                alloc_option(1, err_str as *mut u8) as *mut u8
            }
        }
    }
}

/// Regex.match(rx, str) -> Bool
///
/// Returns true when the pattern matches anywhere in the string.
/// Bool is represented as i8 (1 = true, 0 = false).
#[no_mangle]
pub extern "C" fn mesh_regex_match(rx_ptr: *const u8, s: *const MeshString) -> i8 {
    unsafe {
        let rx = &*(rx_ptr as *const regex::Regex);
        let text = (*s).as_str();
        if rx.is_match(text) { 1 } else { 0 }
    }
}

/// Regex.captures(rx, str) -> Option<List<String>>
///
/// Returns Some(List<String>) where index 0 is the whole match and indices 1..n
/// are capture groups. Returns None when no match is found.
#[no_mangle]
pub extern "C" fn mesh_regex_captures(rx_ptr: *const u8, s: *const MeshString) -> *mut u8 {
    unsafe {
        let rx = &*(rx_ptr as *const regex::Regex);
        let text = (*s).as_str();
        match rx.captures(text) {
            None => alloc_option(1, std::ptr::null_mut()) as *mut u8,
            Some(caps) => {
                let n = caps.len();
                let list = mesh_list_builder_new(n as i64);
                for i in 0..n {
                    let group_str = caps.get(i).map(|m| m.as_str()).unwrap_or("");
                    let ms = mesh_string_new(group_str.as_ptr(), group_str.len() as u64);
                    mesh_list_builder_push(list, ms as u64);
                }
                alloc_option(0, list) as *mut u8
            }
        }
    }
}

/// Regex.replace(rx, str, replacement) -> String
///
/// Replaces all non-overlapping matches with the replacement string.
#[no_mangle]
pub extern "C" fn mesh_regex_replace(
    rx_ptr: *const u8,
    s: *const MeshString,
    replacement: *const MeshString,
) -> *mut MeshString {
    unsafe {
        let rx = &*(rx_ptr as *const regex::Regex);
        let text = (*s).as_str();
        let repl = (*replacement).as_str();
        let result: String = rx.replace_all(text, repl).into_owned();
        mesh_string_new(result.as_ptr(), result.len() as u64)
    }
}

/// Regex.split(rx, str) -> List<String>
///
/// Splits the string by the regex pattern, returning a List<String>.
#[no_mangle]
pub extern "C" fn mesh_regex_split(rx_ptr: *const u8, s: *const MeshString) -> *mut u8 {
    unsafe {
        let rx = &*(rx_ptr as *const regex::Regex);
        let text = (*s).as_str();
        let parts: Vec<&str> = rx.split(text).collect();
        let list = mesh_list_builder_new(parts.len() as i64);
        for part in &parts {
            let ms = mesh_string_new(part.as_ptr(), part.len() as u64);
            mesh_list_builder_push(list, ms as u64);
        }
        list
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::list::mesh_list_length;
    use crate::gc::mesh_rt_init;

    fn ms(s: &str) -> *mut MeshString {
        mesh_string_new(s.as_ptr(), s.len() as u64)
    }

    #[test]
    fn test_regex_from_literal_basic() {
        mesh_rt_init();
        let pat = ms("hello");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        assert!(!rx_ptr.is_null());
        // Verify it actually matches
        let text = ms("hello world");
        let result = mesh_regex_match(rx_ptr as *const u8, text);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_regex_compile_valid() {
        mesh_rt_init();
        let pat = ms("\\d+");
        let result_ptr = mesh_regex_compile(pat);
        unsafe {
            let opt = &*(result_ptr as *const crate::option::MeshOption);
            assert_eq!(opt.tag, 0, "expected Ok tag=0 for valid pattern");
            assert!(!opt.value.is_null());
        }
    }

    #[test]
    fn test_regex_compile_invalid() {
        mesh_rt_init();
        let pat = ms("(unclosed");
        let result_ptr = mesh_regex_compile(pat);
        unsafe {
            let opt = &*(result_ptr as *const crate::option::MeshOption);
            assert_eq!(opt.tag, 1, "expected Err tag=1 for invalid pattern");
        }
    }

    #[test]
    fn test_regex_match_true() {
        mesh_rt_init();
        let pat = ms("\\d+");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        let text = ms("foo123");
        let result = mesh_regex_match(rx_ptr as *const u8, text);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_regex_match_false() {
        mesh_rt_init();
        let pat = ms("^\\d+$");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        let text = ms("foo");
        let result = mesh_regex_match(rx_ptr as *const u8, text);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_regex_captures_some() {
        mesh_rt_init();
        let pat = ms("(\\w+) (\\w+)");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        let text = ms("hello world");
        let result_ptr = mesh_regex_captures(rx_ptr as *const u8, text);
        unsafe {
            let opt = &*(result_ptr as *const crate::option::MeshOption);
            assert_eq!(opt.tag, 0, "expected Some tag=0 for a match");
            let list = opt.value;
            assert_eq!(mesh_list_length(list), 3); // full match + 2 groups
        }
    }

    #[test]
    fn test_regex_captures_none() {
        mesh_rt_init();
        let pat = ms("(\\d+)");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        let text = ms("no digits here");
        let result_ptr = mesh_regex_captures(rx_ptr as *const u8, text);
        unsafe {
            let opt = &*(result_ptr as *const crate::option::MeshOption);
            assert_eq!(opt.tag, 1, "expected None tag=1 for no match");
        }
    }

    #[test]
    fn test_regex_replace() {
        mesh_rt_init();
        let pat = ms("\\d+");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        let text = ms("foo123bar");
        let repl = ms("N");
        let result = mesh_regex_replace(rx_ptr as *const u8, text, repl);
        unsafe {
            assert_eq!((*result).as_str(), "fooNbar");
        }
    }

    #[test]
    fn test_regex_split() {
        mesh_rt_init();
        let pat = ms(",");
        let rx_ptr = mesh_regex_from_literal(pat, 0);
        let text = ms("a,b,c");
        let list = mesh_regex_split(rx_ptr as *const u8, text);
        assert_eq!(mesh_list_length(list), 3);
    }
}
