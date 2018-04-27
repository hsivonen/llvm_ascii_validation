// Copyright 2016 Mozilla Foundation. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(link_llvm_intrinsics, simd_ffi, test, repr_simd)]
#![allow(non_camel_case_types, improper_ctypes)]

#[repr(simd)]
#[derive(Copy, Clone)]
pub struct u8x16(
    u8, u8, u8, u8, u8, u8, u8, u8,
    u8, u8, u8, u8, u8, u8, u8, u8,
);

extern {
    #[link_name = "llvm.x86.sse2.pmovmskb.128"]
    fn x86_mm_movemask_epi8(x: u8x16) -> i32;
}

#[inline(always)]
pub fn mask_ascii(s: u8x16) -> i32 {
    unsafe {
        let signed: u8x16 = ::std::mem::transmute_copy(&s);
        x86_mm_movemask_epi8(signed)
    }
}

#[inline(always)]
pub unsafe fn load16_unaligned(ptr: *const u8) -> u8x16 {
    let mut simd = ::std::mem::uninitialized();
    ::std::ptr::copy_nonoverlapping(ptr, &mut simd as *mut u8x16 as *mut u8, 16);
    simd
}

pub const STRIDE_SIZE: usize = 16;

pub const ALIGNMENT_MASK: usize = 15;

#[inline(always)]
pub fn validate_ascii(slice: &[u8]) -> Option<(u8, usize)> {
    let src = slice.as_ptr();
    let len = slice.len();
    let mut offset = 0usize;
    if STRIDE_SIZE <= len {
        let len_minus_stride = len - STRIDE_SIZE;
        loop {
            let simd = unsafe { load16_unaligned(src.offset(offset as isize)) };
            let mask = mask_ascii(simd);
            if mask != 0 {
                offset += mask.trailing_zeros() as usize;
                let non_ascii = unsafe { *src.offset(offset as isize) };
                return Some((non_ascii, offset));
            }
            offset += STRIDE_SIZE;
            if offset > len_minus_stride {
                break;
            }
        }
    }
    while offset < len {
        let code_unit = slice[offset];
        if code_unit > 127 {
            return Some((code_unit, offset));
        }
        offset += 1;
    }
    None
}

#[inline(never)]
pub fn ascii_valid_up_to(bytes: &[u8]) -> usize {
    match validate_ascii(bytes) {
        None => bytes.len(),
        Some((_, num_valid)) => num_valid,
    }
}

#[test]
fn test() {
    // malloc returns an aligned pointer, so let's use Vec to guarantee
    // alignment.
    let mut v = Vec::new();
    v.extend_from_slice(b"1234567890ABCDEF1234567890ABCDEF1234567890\xFFaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(ascii_valid_up_to(&v[..]), 42);
    assert_eq!(ascii_valid_up_to(&v[1..]), 41);
}

extern crate test;

#[bench]
fn bench(bh: &mut test::Bencher) {
    // The Firefox test used length 100 with 8-byte offset from the start of
    // the jemalloc-returned block and looped 200000 times.
    let b = vec![b'a'; 108];
    bh.iter(|| for _ in 0..200000 {
        test::black_box(ascii_valid_up_to(test::black_box(&b[8..])));
    });
}
