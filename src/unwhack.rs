// Copyright 2024-2026 by Michael Stroucken
use crate::constants::*;

/// uncompress a section of data
///
/// Takes data in `src` and uncompresses to a [`Vec<u8>`]
/// up to `ndst` bytes. Returns [`String`] for errors.
///
/// # Errors
///
/// If the output exceeds the specified size or the stream
/// cannot be correctly interpreted
pub fn unwhack(src: &[u8], ndst: usize) -> Result<Vec<u8>, String> {
    let mut dst: Vec<u8> = Vec::with_capacity(ndst);
    let mut current_dest_pos = 0;
    let mut current_source_pos = 0;
    let mut read_bits_count: u32 = 0;
    let mut read_bits: usize = 0;
    let mut over_bits_count = 0;
    let max_source_pos = src.len();
    let max_dest_pos = ndst;
    let mut lithist: usize = !0;

    while current_source_pos < max_source_pos || read_bits_count - over_bits_count >= MIN_DECODE {
        while read_bits_count <= 24 {
            read_bits <<= 8;
            if current_source_pos < max_source_pos {
                read_bits |= src[current_source_pos] as usize;
                current_source_pos += 1;
            } else {
                over_bits_count += 8;
            }
            read_bits_count += 8;
        }
        /*
        literal
         */
        let mut len = LENVAL[read_bits >> (read_bits_count - 5) & 0x1f] as usize;
        if len == 0 {
            let mut lit;
            if lithist & 0xf != 0 {
                read_bits_count -= 9;
                lit = (read_bits >> read_bits_count & 0xff) as u8;
            } else {
                read_bits_count -= 8;
                lit = (read_bits >> read_bits_count & 0x7f) as u8;
                if (lit) < 32 {
                    if (lit) < 24 {
                        read_bits_count -= 2;
                        lit = ((lit) << 2) | (read_bits >> read_bits_count & 3) as u8;
                    } else {
                        read_bits_count -= 3;
                        lit = ((lit) << 3) | (read_bits >> read_bits_count & 7) as u8;
                    }
                    lit -= 64;
                }
            }
            if current_dest_pos >= max_dest_pos {
                return Err(String::from("too much output"));
            }

            dst.push(lit);
            current_dest_pos += 1;
            lithist = (lithist << 1) | if !(32..=127).contains(&lit) { 1 } else { 0 };
        } else {
            /*
            length
             */
            if len < 255 {
                read_bits_count -= LENBITS[len] as u32;
            } else {
                read_bits_count -= D_BIG_LEN_BITS;
                let mut code = ((read_bits >> read_bits_count & (((1) << D_BIG_LEN_BITS) - 1))
                    - D_BIG_LEN_CODE as usize) as u32;
                len = DMAX_FAST_LEN;
                let mut use_0 = D_BIG_LEN_BASE;
                let mut bits = D_BIG_LEN_BITS & 1 ^ 1;
                while code >= use_0 {
                    if read_bits_count == 0 {
                        return Err(String::from("len out of range"));
                    }
                    len += use_0 as usize;
                    code -= use_0;
                    code <<= 1;
                    read_bits_count -= 1;

                    code |= (read_bits >> read_bits_count & 1) as u32;
                    use_0 <<= bits;
                    bits ^= 1;
                }
                len += code as usize;
                while read_bits_count <= 24 {
                    read_bits <<= 8;
                    if current_source_pos < max_source_pos {
                        read_bits |= src[current_source_pos] as usize;
                        current_source_pos += 1;
                    } else {
                        over_bits_count += 8;
                    }
                    read_bits_count += 8;
                }
            }
            /*
            offset
             */
            read_bits_count -= 4;
            let mut bits = (read_bits >> read_bits_count & 0xf) as u32;
            let mut off = OFFBASE[bits as usize] as usize;
            bits = OFFBITS[bits as usize] as u32;
            read_bits_count -= bits;
            off |= (read_bits >> read_bits_count) & (((1) << bits) - 1);
            off += 1;
            if off > current_dest_pos {
                return Err(format!(
                    "offset out of range: off={off} d={current_dest_pos} len={len} nbits={read_bits_count}",
                ));
            }

            if current_dest_pos + len > max_dest_pos {
                return Err(String::from("len out of range"));
            }

            let s = current_dest_pos - off;

            // can't use extend_from_within, because the vector grows with data that will be used
            //dst.extend_from_within(s..s + len);
            let mut i = 0;
            while i < len {
                dst.push(dst[s + i]);
                i += 1;
            }

            current_dest_pos += len;
        }
    }
    if read_bits_count < over_bits_count {
        return Err(String::from("compressed data overrun"));
    }

    //len = dpos;
    //assert_eq!(len, ndst);
    Ok(dst)
}
