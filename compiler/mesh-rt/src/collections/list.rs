//! GC-managed immutable List for the Mesh runtime.
//!
//! A MeshList stores elements as uniform 8-byte (`u64`) values in a contiguous
//! GC-allocated buffer. Layout: `{ len: u64, cap: u64, data: [u64; cap] }`.
//!
//! All mutation operations (append, tail, concat, etc.) return a NEW list,
//! preserving immutability semantics.

use crate::gc::mesh_gc_alloc_actor;
use crate::option::alloc_option;
use std::ptr;

/// Header size: len (8 bytes) + cap (8 bytes) = 16 bytes.
const HEADER_SIZE: usize = 16;

/// Byte size of one element.
const ELEM_SIZE: usize = 8;

// ── Internal helpers ──────────────────────────────────────────────────

/// Read the length field from a list pointer.
unsafe fn list_len(list: *const u8) -> u64 {
    *(list as *const u64)
}

/// Read the capacity field from a list pointer.
#[allow(dead_code)]
unsafe fn list_cap(list: *const u8) -> u64 {
    *((list as *const u64).add(1))
}

/// Get a pointer to the data region (past the header).
unsafe fn list_data(list: *const u8) -> *const u64 {
    (list as *const u64).add(2)
}

/// Get a mutable pointer to the data region.
unsafe fn list_data_mut(list: *mut u8) -> *mut u64 {
    (list as *mut u64).add(2)
}

/// Allocate a new list with the given capacity, length set to 0.
unsafe fn alloc_list(cap: u64) -> *mut u8 {
    let total = HEADER_SIZE + (cap as usize) * ELEM_SIZE;
    let p = mesh_gc_alloc_actor(total as u64, 8);
    // len = 0, cap = cap
    *(p as *mut u64) = 0;
    *((p as *mut u64).add(1)) = cap;
    p
}

/// Allocate a new list with the given length and capacity, copying `len` elements from `src`.
unsafe fn alloc_list_from(src: *const u64, len: u64, cap: u64) -> *mut u8 {
    let p = alloc_list(cap);
    *(p as *mut u64) = len;
    if len > 0 {
        ptr::copy_nonoverlapping(src, list_data_mut(p), len as usize);
    }
    p
}

/// Allocate a 2-element tuple on the GC heap matching Mesh's tuple layout.
/// Layout: { u64 len=2, u64 elem0, u64 elem1 }
pub(crate) unsafe fn alloc_pair(a: u64, b: u64) -> *mut u8 {
    let total = 8 + 2 * 8; // len field + 2 elements
    let p = mesh_gc_alloc_actor(total as u64, 8);
    *(p as *mut u64) = 2;           // len = 2
    *((p as *mut u64).add(1)) = a;  // first element
    *((p as *mut u64).add(2)) = b;  // second element
    p
}

// ── Public API ────────────────────────────────────────────────────────

/// Create an empty list.
#[no_mangle]
pub extern "C" fn mesh_list_new() -> *mut u8 {
    unsafe { alloc_list(0) }
}

/// Return the number of elements in the list.
#[no_mangle]
pub extern "C" fn mesh_list_length(list: *mut u8) -> i64 {
    unsafe { list_len(list) as i64 }
}

/// Return a NEW list with `element` appended at the end.
#[no_mangle]
pub extern "C" fn mesh_list_append(list: *mut u8, element: u64) -> *mut u8 {
    unsafe {
        let len = list_len(list);
        let new_cap = len + 1;
        let new_list = alloc_list(new_cap);
        *(new_list as *mut u64) = new_cap; // len = old len + 1
        if len > 0 {
            ptr::copy_nonoverlapping(list_data(list), list_data_mut(new_list), len as usize);
        }
        *list_data_mut(new_list).add(len as usize) = element;
        new_list
    }
}

/// Return the first element. Panics if empty.
#[no_mangle]
pub extern "C" fn mesh_list_head(list: *mut u8) -> u64 {
    unsafe {
        let len = list_len(list);
        if len == 0 {
            panic!("mesh_list_head: empty list");
        }
        *list_data(list)
    }
}

/// Return a NEW list without the first element. Panics if empty.
#[no_mangle]
pub extern "C" fn mesh_list_tail(list: *mut u8) -> *mut u8 {
    unsafe {
        let len = list_len(list);
        if len == 0 {
            panic!("mesh_list_tail: empty list");
        }
        let new_len = len - 1;
        alloc_list_from(list_data(list).add(1), new_len, new_len)
    }
}

/// Get the element at `index`. Panics if out of bounds.
#[no_mangle]
pub extern "C" fn mesh_list_get(list: *mut u8, index: i64) -> u64 {
    unsafe {
        let len = list_len(list);
        if index < 0 || index as u64 >= len {
            panic!(
                "mesh_list_get: index {} out of bounds (len {})",
                index, len
            );
        }
        *list_data(list).add(index as usize)
    }
}

/// Concatenate two lists into a NEW list.
#[no_mangle]
pub extern "C" fn mesh_list_concat(a: *mut u8, b: *mut u8) -> *mut u8 {
    unsafe {
        let a_len = list_len(a);
        let b_len = list_len(b);
        let new_len = a_len + b_len;
        let new_list = alloc_list(new_len);
        *(new_list as *mut u64) = new_len;
        if a_len > 0 {
            ptr::copy_nonoverlapping(list_data(a), list_data_mut(new_list), a_len as usize);
        }
        if b_len > 0 {
            ptr::copy_nonoverlapping(
                list_data(b),
                list_data_mut(new_list).add(a_len as usize),
                b_len as usize,
            );
        }
        new_list
    }
}

/// Return a reversed copy of the list.
#[no_mangle]
pub extern "C" fn mesh_list_reverse(list: *mut u8) -> *mut u8 {
    unsafe {
        let len = list_len(list);
        let new_list = alloc_list(len);
        *(new_list as *mut u64) = len;
        let src = list_data(list);
        let dst = list_data_mut(new_list);
        for i in 0..len as usize {
            *dst.add(i) = *src.add(len as usize - 1 - i);
        }
        new_list
    }
}

/// Apply a closure to each element, returning a new list.
///
/// If `env_ptr` is null, `fn_ptr` is called as `fn(element) -> result`.
/// If `env_ptr` is non-null, `fn_ptr` is called as `fn(env_ptr, element) -> result`.
#[no_mangle]
pub extern "C" fn mesh_list_map(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> *mut u8 {
    type BareFn = unsafe extern "C" fn(u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64) -> u64;

    unsafe {
        let len = list_len(list);
        let new_list = alloc_list(len);
        *(new_list as *mut u64) = len;
        let src = list_data(list);
        let dst = list_data_mut(new_list);

        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                *dst.add(i) = f(*src.add(i));
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                *dst.add(i) = f(env_ptr, *src.add(i));
            }
        }
        new_list
    }
}

/// Keep elements where the closure returns non-zero (true).
#[no_mangle]
pub extern "C" fn mesh_list_filter(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> *mut u8 {
    type BareFn = unsafe extern "C" fn(u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64) -> u64;

    unsafe {
        let len = list_len(list);
        // Allocate worst case, then shrink.
        let temp = alloc_list(len);
        let src = list_data(list);
        let dst = list_data_mut(temp);
        let mut count = 0u64;

        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                let elem = *src.add(i);
                if f(elem) != 0 {
                    *dst.add(count as usize) = elem;
                    count += 1;
                }
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                let elem = *src.add(i);
                if f(env_ptr, elem) != 0 {
                    *dst.add(count as usize) = elem;
                    count += 1;
                }
            }
        }

        // Set actual length.
        *(temp as *mut u64) = count;
        temp
    }
}

/// Fold left over the list with an accumulator.
///
/// If `env_ptr` is null: `fn_ptr(acc, element) -> acc`
/// If `env_ptr` is non-null: `fn_ptr(env_ptr, acc, element) -> acc`
#[no_mangle]
pub extern "C" fn mesh_list_reduce(
    list: *mut u8,
    init: u64,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> u64 {
    type BareFn = unsafe extern "C" fn(u64, u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64, u64) -> u64;

    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        let mut acc = init;

        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                acc = f(acc, *src.add(i));
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                acc = f(env_ptr, acc, *src.add(i));
            }
        }
        acc
    }
}

/// Create a list with pre-allocated capacity for N elements.
/// Length starts at 0. Used by for-in codegen for O(N) result building.
#[no_mangle]
pub extern "C" fn mesh_list_builder_new(capacity: i64) -> *mut u8 {
    unsafe { alloc_list(capacity.max(0) as u64) }
}

/// Push an element to a list builder (in-place mutation, O(1)).
/// SAFETY: Only valid during construction before the list is shared.
/// Increments len and writes element at data[len].
#[no_mangle]
pub extern "C" fn mesh_list_builder_push(list: *mut u8, element: u64) {
    unsafe {
        let len = list_len(list) as usize;
        let data = list_data_mut(list);
        *data.add(len) = element;
        *(list as *mut u64) = (len + 1) as u64;
    }
}

/// Create a list from an array of u64 elements.
#[no_mangle]
pub extern "C" fn mesh_list_from_array(data: *const u64, count: i64) -> *mut u8 {
    unsafe {
        let count = count.max(0) as u64;
        alloc_list_from(data, count, count)
    }
}

/// Compare two lists for equality using an element-comparison callback.
///
/// `elem_eq` is a bare function pointer `fn(u64, u64) -> i8` that returns 1
/// if two elements are equal, 0 otherwise. Returns 1 if lists are equal, 0 if not.
#[no_mangle]
pub extern "C" fn mesh_list_eq(
    list_a: *mut u8,
    list_b: *mut u8,
    elem_eq: *mut u8,
) -> i8 {
    type ElemEq = unsafe extern "C" fn(u64, u64) -> i8;

    unsafe {
        let len_a = list_len(list_a);
        let len_b = list_len(list_b);
        if len_a != len_b {
            return 0;
        }
        let data_a = list_data(list_a);
        let data_b = list_data(list_b);
        let f: ElemEq = std::mem::transmute(elem_eq);
        for i in 0..len_a as usize {
            if f(*data_a.add(i), *data_b.add(i)) == 0 {
                return 0;
            }
        }
        1
    }
}

/// Compare two lists lexicographically using an element-comparison callback.
///
/// `elem_cmp` is a bare function pointer `fn(u64, u64) -> i64` that returns
/// negative if a < b, 0 if equal, positive if a > b. Returns negative/0/positive
/// for the lexicographic ordering of the two lists.
#[no_mangle]
pub extern "C" fn mesh_list_compare(
    list_a: *mut u8,
    list_b: *mut u8,
    elem_cmp: *mut u8,
) -> i64 {
    type ElemCmp = unsafe extern "C" fn(u64, u64) -> i64;

    unsafe {
        let len_a = list_len(list_a) as usize;
        let len_b = list_len(list_b) as usize;
        let data_a = list_data(list_a);
        let data_b = list_data(list_b);
        let f: ElemCmp = std::mem::transmute(elem_cmp);
        let min_len = len_a.min(len_b);
        for i in 0..min_len {
            let cmp = f(*data_a.add(i), *data_b.add(i));
            if cmp != 0 {
                return cmp;
            }
        }
        if len_a < len_b {
            -1
        } else if len_a > len_b {
            1
        } else {
            0
        }
    }
}

/// Convert a list to a human-readable MeshString: `[elem1, elem2, ...]`.
///
/// `elem_to_str` is a bare function pointer `fn(u64) -> *mut u8` that converts
/// each element (stored as a uniform u64) to a MeshString pointer. The MIR
/// lowerer passes the appropriate runtime to_string function (e.g.,
/// `mesh_int_to_string` for `List<Int>`).
#[no_mangle]
pub extern "C" fn mesh_list_to_string(
    list: *mut u8,
    elem_to_str: *mut u8,
) -> *mut u8 {
    type ElemToStr = unsafe extern "C" fn(u64) -> *mut u8;

    unsafe {
        let len = list_len(list) as usize;
        let data = list_data(list);
        let f: ElemToStr = std::mem::transmute(elem_to_str);

        // Build the result string piece by piece using mesh_string_concat.
        let mut result = crate::string::mesh_string_new(b"[".as_ptr(), 1) as *mut u8;
        for i in 0..len {
            if i > 0 {
                let sep = crate::string::mesh_string_new(b", ".as_ptr(), 2) as *mut u8;
                result = crate::string::mesh_string_concat(
                    result as *const crate::string::MeshString,
                    sep as *const crate::string::MeshString,
                ) as *mut u8;
            }
            let elem_str = f(*data.add(i));
            result = crate::string::mesh_string_concat(
                result as *const crate::string::MeshString,
                elem_str as *const crate::string::MeshString,
            ) as *mut u8;
        }
        let close = crate::string::mesh_string_new(b"]".as_ptr(), 1) as *mut u8;
        result = crate::string::mesh_string_concat(
            result as *const crate::string::MeshString,
            close as *const crate::string::MeshString,
        ) as *mut u8;
        result
    }
}

/// Sort a list using a user-provided comparator function.
///
/// The comparator returns an i64: negative = less, 0 = equal, positive = greater.
/// Returns a NEW sorted list (immutability preserved).
///
/// If `env_ptr` is null, `fn_ptr` is called as `fn(a, b) -> i64`.
/// If `env_ptr` is non-null, `fn_ptr` is called as `fn(env_ptr, a, b) -> i64`.
#[no_mangle]
pub extern "C" fn mesh_list_sort(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> *mut u8 {
    type BareFn = unsafe extern "C" fn(u64, u64) -> i64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64, u64) -> i64;

    unsafe {
        let len = list_len(list);
        if len <= 1 {
            // Return a copy to preserve immutability semantics.
            return alloc_list_from(list_data(list), len, len);
        }
        // Copy elements into a mutable Vec for sorting.
        let src = list_data(list);
        let mut elements: Vec<u64> = Vec::with_capacity(len as usize);
        for i in 0..len as usize {
            elements.push(*src.add(i));
        }
        // Sort using the comparator.
        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            elements.sort_by(|a, b| {
                let cmp = f(*a, *b);
                if cmp < 0 {
                    std::cmp::Ordering::Less
                } else if cmp > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            elements.sort_by(|a, b| {
                let cmp = f(env_ptr, *a, *b);
                if cmp < 0 {
                    std::cmp::Ordering::Less
                } else if cmp > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }
        // Allocate new list with sorted elements.
        let new_list = alloc_list(len);
        *(new_list as *mut u64) = len;
        let dst = list_data_mut(new_list);
        for (i, elem) in elements.iter().enumerate() {
            *dst.add(i) = *elem;
        }
        new_list
    }
}

/// Find the first element matching a predicate. Returns MeshOption
/// (tag 0 = Some with element, tag 1 = None).
///
/// If `env_ptr` is null, `fn_ptr` is called as `fn(elem) -> u64` (nonzero = true).
/// If `env_ptr` is non-null, `fn_ptr` is called as `fn(env_ptr, elem) -> u64`.
#[no_mangle]
pub extern "C" fn mesh_list_find(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> *mut u8 {
    type BareFn = unsafe extern "C" fn(u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64) -> u64;

    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                let elem = *src.add(i);
                if f(elem) != 0 {
                    return alloc_option(0, elem as *mut u8) as *mut u8; // Some(elem)
                }
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                let elem = *src.add(i);
                if f(env_ptr, elem) != 0 {
                    return alloc_option(0, elem as *mut u8) as *mut u8; // Some(elem)
                }
            }
        }
        alloc_option(1, std::ptr::null_mut()) as *mut u8 // None
    }
}

/// Test if any element matches a predicate.
///
/// Returns 1 (true) if at least one element matches, 0 (false) otherwise.
/// Short-circuits on first match.
#[no_mangle]
pub extern "C" fn mesh_list_any(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> i8 {
    type BareFn = unsafe extern "C" fn(u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64) -> u64;

    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                if f(*src.add(i)) != 0 {
                    return 1;
                }
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                if f(env_ptr, *src.add(i)) != 0 {
                    return 1;
                }
            }
        }
        0
    }
}

/// Test if all elements match a predicate.
///
/// Returns 1 (true) if every element matches, 0 (false) otherwise.
/// Short-circuits on first non-match.
#[no_mangle]
pub extern "C" fn mesh_list_all(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> i8 {
    type BareFn = unsafe extern "C" fn(u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64) -> u64;

    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                if f(*src.add(i)) == 0 {
                    return 0;
                }
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                if f(env_ptr, *src.add(i)) == 0 {
                    return 0;
                }
            }
        }
        1
    }
}

/// Test if a list contains an element using raw u64 equality.
///
/// Returns 1 if found, 0 if not. Works correctly for Int and Bool.
/// For String lists, the codegen emits `mesh_list_contains_str` instead.
#[no_mangle]
pub extern "C" fn mesh_list_contains(list: *mut u8, elem: u64) -> i8 {
    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        for i in 0..len as usize {
            if *src.add(i) == elem {
                return 1;
            }
        }
        0
    }
}

/// Test if a list of strings contains a given string using content equality.
///
/// Uses `mesh_string_eq` for byte-by-byte comparison, so two distinct string
/// allocations with identical content compare equal.  The codegen redirects
/// `List.contains` calls whose element type is String to this function.
#[no_mangle]
pub extern "C" fn mesh_list_contains_str(
    list: *mut u8,
    elem: *const crate::string::MeshString,
) -> i8 {
    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        for i in 0..len as usize {
            let item = *src.add(i) as *const crate::string::MeshString;
            if crate::string::mesh_string_eq(item, elem) != 0 {
                return 1;
            }
        }
        0
    }
}

/// Zip two lists into a list of 2-tuples, truncated to the shorter length.
#[no_mangle]
pub extern "C" fn mesh_list_zip(a: *mut u8, b: *mut u8) -> *mut u8 {
    unsafe {
        let len_a = list_len(a);
        let len_b = list_len(b);
        let len = len_a.min(len_b);

        let result = alloc_list(len);
        *(result as *mut u64) = len;
        let src_a = list_data(a);
        let src_b = list_data(b);
        let dst = list_data_mut(result);

        for i in 0..len as usize {
            let pair = alloc_pair(*src_a.add(i), *src_b.add(i));
            *dst.add(i) = pair as u64;
        }
        result
    }
}

/// Apply a closure to each element that returns a list, then flatten all results.
///
/// If `env_ptr` is null, `fn_ptr` is called as `fn(element) -> list_ptr_as_u64`.
/// If `env_ptr` is non-null, `fn_ptr` is called as `fn(env_ptr, element) -> list_ptr_as_u64`.
#[no_mangle]
pub extern "C" fn mesh_list_flat_map(
    list: *mut u8,
    fn_ptr: *mut u8,
    env_ptr: *mut u8,
) -> *mut u8 {
    type BareFn = unsafe extern "C" fn(u64) -> u64;
    type ClosureFn = unsafe extern "C" fn(*mut u8, u64) -> u64;

    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        let mut all_elems: Vec<u64> = Vec::new();

        if env_ptr.is_null() {
            let f: BareFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                let sub_list = f(*src.add(i)) as *mut u8;
                let sub_len = list_len(sub_list) as usize;
                let sub_data = list_data(sub_list);
                for j in 0..sub_len {
                    all_elems.push(*sub_data.add(j));
                }
            }
        } else {
            let f: ClosureFn = std::mem::transmute(fn_ptr);
            for i in 0..len as usize {
                let sub_list = f(env_ptr, *src.add(i)) as *mut u8;
                let sub_len = list_len(sub_list) as usize;
                let sub_data = list_data(sub_list);
                for j in 0..sub_len {
                    all_elems.push(*sub_data.add(j));
                }
            }
        }

        let result_len = all_elems.len() as u64;
        let result = alloc_list(result_len);
        *(result as *mut u64) = result_len;
        let dst = list_data_mut(result);
        for (i, elem) in all_elems.iter().enumerate() {
            *dst.add(i) = *elem;
        }
        result
    }
}

/// Flatten a list of lists into a single list.
///
/// Each element of the outer list is treated as a list pointer (stored as u64).
#[no_mangle]
pub extern "C" fn mesh_list_flatten(list: *mut u8) -> *mut u8 {
    unsafe {
        let outer_len = list_len(list) as usize;
        let outer_data = list_data(list);
        let mut all_elems: Vec<u64> = Vec::new();

        for i in 0..outer_len {
            let sub_list = *outer_data.add(i) as *mut u8;
            let sub_len = list_len(sub_list) as usize;
            let sub_data = list_data(sub_list);
            for j in 0..sub_len {
                all_elems.push(*sub_data.add(j));
            }
        }

        let result_len = all_elems.len() as u64;
        let result = alloc_list(result_len);
        *(result as *mut u64) = result_len;
        let dst = list_data_mut(result);
        for (i, elem) in all_elems.iter().enumerate() {
            *dst.add(i) = *elem;
        }
        result
    }
}

/// Create a list of (index, element) tuples from a list.
#[no_mangle]
pub extern "C" fn mesh_list_enumerate(list: *mut u8) -> *mut u8 {
    unsafe {
        let len = list_len(list);
        let src = list_data(list);
        let result = alloc_list(len);
        *(result as *mut u64) = len;
        let dst = list_data_mut(result);

        for i in 0..len as usize {
            let pair = alloc_pair(i as u64, *src.add(i));
            *dst.add(i) = pair as u64;
        }
        result
    }
}

/// Return a new list with the first `n` elements.
/// Clamps `n` to [0, len].
#[no_mangle]
pub extern "C" fn mesh_list_take(list: *mut u8, n: i64) -> *mut u8 {
    unsafe {
        let len = list_len(list);
        let actual_n = (n.max(0) as u64).min(len);
        alloc_list_from(list_data(list), actual_n, actual_n)
    }
}

/// Return a new list with the first `n` elements removed.
/// Clamps `n` to [0, len].
#[no_mangle]
pub extern "C" fn mesh_list_drop(list: *mut u8, n: i64) -> *mut u8 {
    unsafe {
        let len = list_len(list);
        let actual_n = (n.max(0) as u64).min(len);
        let remaining = len - actual_n;
        alloc_list_from(list_data(list).add(actual_n as usize), remaining, remaining)
    }
}

/// Return the last element of the list. Panics if empty.
#[no_mangle]
pub extern "C" fn mesh_list_last(list: *mut u8) -> u64 {
    unsafe {
        let len = list_len(list);
        if len == 0 {
            panic!("mesh_list_last: empty list");
        }
        *list_data(list).add(len as usize - 1)
    }
}

/// Return the element at index `n`. Panics if out of bounds.
/// (Alias for get, used by List.nth module-qualified access.)
#[no_mangle]
pub extern "C" fn mesh_list_nth(list: *mut u8, index: i64) -> u64 {
    mesh_list_get(list, index)
}

// ── Iterator handle ───────────────────────────────────────────────────

/// Internal iterator state for List iteration.
#[repr(C)]
struct ListIterator {
    tag: u8,
    list: *mut u8,
    index: i64,
    length: i64,
}

/// Create a new iterator handle for a list.
#[no_mangle]
pub extern "C" fn mesh_list_iter_new(list: *mut u8) -> *mut u8 {
    unsafe {
        let len = mesh_list_length(list);
        let iter = mesh_gc_alloc_actor(
            std::mem::size_of::<ListIterator>() as u64,
            std::mem::align_of::<ListIterator>() as u64,
        ) as *mut ListIterator;
        (*iter).tag = 0; // ITER_TAG_LIST
        (*iter).list = list;
        (*iter).index = 0;
        (*iter).length = len;
        iter as *mut u8
    }
}

/// Advance the list iterator, returning Option (tag 0 = Some, tag 1 = None).
#[no_mangle]
pub extern "C" fn mesh_list_iter_next(iter_ptr: *mut u8) -> *mut u8 {
    unsafe {
        let iter = iter_ptr as *mut ListIterator;
        if (*iter).index >= (*iter).length {
            alloc_option(1, std::ptr::null_mut()) as *mut u8
        } else {
            let elem = mesh_list_get((*iter).list, (*iter).index);
            (*iter).index += 1;
            alloc_option(0, elem as usize as *mut u8) as *mut u8
        }
    }
}

/// Iter.from(collection) -- creates an iterator handle from a List.
/// For Phase 76, this is equivalent to mesh_list_iter_new.
/// Future phases can add type-tag dispatch for Map/Set/Range.
#[no_mangle]
pub extern "C" fn mesh_iter_from(collection: *mut u8) -> *mut u8 {
    // Delegate to list iterator creation.
    mesh_list_iter_new(collection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gc::mesh_rt_init;

    #[test]
    fn test_list_new_is_empty() {
        mesh_rt_init();
        let list = mesh_list_new();
        assert_eq!(mesh_list_length(list), 0);
    }

    #[test]
    fn test_list_append_and_length() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 10);
        let list = mesh_list_append(list, 20);
        let list = mesh_list_append(list, 30);
        assert_eq!(mesh_list_length(list), 3);
    }

    #[test]
    fn test_list_head_tail() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 1);
        let list = mesh_list_append(list, 2);
        let list = mesh_list_append(list, 3);
        assert_eq!(mesh_list_head(list), 1);
        let tail = mesh_list_tail(list);
        assert_eq!(mesh_list_length(tail), 2);
        assert_eq!(mesh_list_head(tail), 2);
    }

    #[test]
    fn test_list_get() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 100);
        let list = mesh_list_append(list, 200);
        let list = mesh_list_append(list, 300);
        assert_eq!(mesh_list_get(list, 0), 100);
        assert_eq!(mesh_list_get(list, 1), 200);
        assert_eq!(mesh_list_get(list, 2), 300);
    }

    #[test]
    fn test_list_concat() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 3);
        let b = mesh_list_append(b, 4);
        let c = mesh_list_concat(a, b);
        assert_eq!(mesh_list_length(c), 4);
        assert_eq!(mesh_list_get(c, 0), 1);
        assert_eq!(mesh_list_get(c, 3), 4);
    }

    #[test]
    fn test_list_reverse() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 1);
        let list = mesh_list_append(list, 2);
        let list = mesh_list_append(list, 3);
        let rev = mesh_list_reverse(list);
        assert_eq!(mesh_list_get(rev, 0), 3);
        assert_eq!(mesh_list_get(rev, 1), 2);
        assert_eq!(mesh_list_get(rev, 2), 1);
    }

    #[test]
    fn test_list_map() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 1);
        let list = mesh_list_append(list, 2);
        let list = mesh_list_append(list, 3);

        unsafe extern "C" fn double(x: u64) -> u64 {
            x * 2
        }

        let mapped = mesh_list_map(list, double as *mut u8, std::ptr::null_mut());
        assert_eq!(mesh_list_length(mapped), 3);
        assert_eq!(mesh_list_get(mapped, 0), 2);
        assert_eq!(mesh_list_get(mapped, 1), 4);
        assert_eq!(mesh_list_get(mapped, 2), 6);
    }

    #[test]
    fn test_list_filter() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 1);
        let list = mesh_list_append(list, 2);
        let list = mesh_list_append(list, 3);
        let list = mesh_list_append(list, 4);

        // Keep only even numbers (value % 2 == 0).
        unsafe extern "C" fn is_even(x: u64) -> u64 {
            if x % 2 == 0 { 1 } else { 0 }
        }

        let filtered = mesh_list_filter(list, is_even as *mut u8, std::ptr::null_mut());
        assert_eq!(mesh_list_length(filtered), 2);
        assert_eq!(mesh_list_get(filtered, 0), 2);
        assert_eq!(mesh_list_get(filtered, 1), 4);
    }

    #[test]
    fn test_list_reduce() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 1);
        let list = mesh_list_append(list, 2);
        let list = mesh_list_append(list, 3);

        unsafe extern "C" fn add(acc: u64, x: u64) -> u64 {
            acc + x
        }

        let sum = mesh_list_reduce(list, 0, add as *mut u8, std::ptr::null_mut());
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_list_map_with_closure() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 10);
        let list = mesh_list_append(list, 20);

        // Simulate a closure with an environment: add the value stored at env_ptr.
        unsafe extern "C" fn add_env(env: *mut u8, x: u64) -> u64 {
            let offset = *(env as *const u64);
            x + offset
        }

        // Create a fake "environment" that holds the value 5.
        let mut env_val: u64 = 5;
        let env_ptr = &mut env_val as *mut u64 as *mut u8;

        let mapped = mesh_list_map(list, add_env as *mut u8, env_ptr);
        assert_eq!(mesh_list_get(mapped, 0), 15);
        assert_eq!(mesh_list_get(mapped, 1), 25);
    }

    #[test]
    fn test_list_from_array() {
        mesh_rt_init();
        let data: [u64; 3] = [10, 20, 30];
        let list = mesh_list_from_array(data.as_ptr(), 3);
        assert_eq!(mesh_list_length(list), 3);
        assert_eq!(mesh_list_get(list, 0), 10);
        assert_eq!(mesh_list_get(list, 2), 30);
    }

    #[test]
    fn test_list_empty_reverse() {
        mesh_rt_init();
        let list = mesh_list_new();
        let rev = mesh_list_reverse(list);
        assert_eq!(mesh_list_length(rev), 0);
    }

    #[test]
    fn test_list_reduce_empty() {
        mesh_rt_init();
        let list = mesh_list_new();

        unsafe extern "C" fn add(acc: u64, x: u64) -> u64 {
            acc + x
        }

        let result = mesh_list_reduce(list, 42, add as *mut u8, std::ptr::null_mut());
        assert_eq!(result, 42); // Initial value returned unchanged.
    }

    #[test]
    fn test_list_to_string() {
        mesh_rt_init();
        let list = mesh_list_new();
        let list = mesh_list_append(list, 1);
        let list = mesh_list_append(list, 2);
        let list = mesh_list_append(list, 3);

        let result = mesh_list_to_string(
            list,
            crate::string::mesh_int_to_string as *mut u8,
        );
        let s = unsafe { &*(result as *const crate::string::MeshString) };
        let text = unsafe { s.as_str() };
        assert_eq!(text, "[1, 2, 3]");
    }

    #[test]
    fn test_list_eq_same() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let a = mesh_list_append(a, 3);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 1);
        let b = mesh_list_append(b, 2);
        let b = mesh_list_append(b, 3);

        unsafe extern "C" fn int_eq(a: u64, b: u64) -> i8 {
            if a == b { 1 } else { 0 }
        }

        assert_eq!(mesh_list_eq(a, b, int_eq as *mut u8), 1);
    }

    #[test]
    fn test_list_eq_different() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 1);
        let b = mesh_list_append(b, 3);

        unsafe extern "C" fn int_eq(a: u64, b: u64) -> i8 {
            if a == b { 1 } else { 0 }
        }

        assert_eq!(mesh_list_eq(a, b, int_eq as *mut u8), 0);
    }

    #[test]
    fn test_list_eq_different_length() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 1);

        unsafe extern "C" fn int_eq(a: u64, b: u64) -> i8 {
            if a == b { 1 } else { 0 }
        }

        assert_eq!(mesh_list_eq(a, b, int_eq as *mut u8), 0);
    }

    #[test]
    fn test_list_compare_less() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 1);
        let b = mesh_list_append(b, 3);

        unsafe extern "C" fn int_cmp(a: u64, b: u64) -> i64 {
            (a as i64) - (b as i64)
        }

        assert!(mesh_list_compare(a, b, int_cmp as *mut u8) < 0);
    }

    #[test]
    fn test_list_compare_equal() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 1);
        let b = mesh_list_append(b, 2);

        unsafe extern "C" fn int_cmp(a: u64, b: u64) -> i64 {
            (a as i64) - (b as i64)
        }

        assert_eq!(mesh_list_compare(a, b, int_cmp as *mut u8), 0);
    }

    #[test]
    fn test_list_compare_length() {
        mesh_rt_init();
        let a = mesh_list_new();
        let a = mesh_list_append(a, 1);
        let a = mesh_list_append(a, 2);
        let b = mesh_list_new();
        let b = mesh_list_append(b, 1);
        let b = mesh_list_append(b, 2);
        let b = mesh_list_append(b, 3);

        unsafe extern "C" fn int_cmp(a: u64, b: u64) -> i64 {
            (a as i64) - (b as i64)
        }

        assert!(mesh_list_compare(a, b, int_cmp as *mut u8) < 0);
    }

    #[test]
    fn test_list_to_string_empty() {
        mesh_rt_init();
        let list = mesh_list_new();

        let result = mesh_list_to_string(
            list,
            crate::string::mesh_int_to_string as *mut u8,
        );
        let s = unsafe { &*(result as *const crate::string::MeshString) };
        let text = unsafe { s.as_str() };
        assert_eq!(text, "[]");
    }

    #[test]
    fn test_list_builder_new_empty() {
        mesh_rt_init();
        let list = mesh_list_builder_new(0);
        assert_eq!(mesh_list_length(list), 0);
    }

    #[test]
    fn test_list_builder_new_has_zero_length() {
        mesh_rt_init();
        let list = mesh_list_builder_new(3);
        assert_eq!(mesh_list_length(list), 0);
    }

    #[test]
    fn test_list_builder_push_three_elements() {
        mesh_rt_init();
        let list = mesh_list_builder_new(3);
        mesh_list_builder_push(list, 10);
        mesh_list_builder_push(list, 20);
        mesh_list_builder_push(list, 30);
        assert_eq!(mesh_list_length(list), 3);
        assert_eq!(mesh_list_get(list, 0), 10);
        assert_eq!(mesh_list_get(list, 1), 20);
        assert_eq!(mesh_list_get(list, 2), 30);
    }
}
