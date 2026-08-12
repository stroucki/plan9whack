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
        // try to read 4 bytes
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

        // look at top 5 bits read, if two top bits are 00 or 01, it is a literal,
        // otherwise it is a (length, offset) pair
        let mut len = LENVAL[read_bits >> (read_bits_count - 5) & 0x1f] as usize;

        if len == 0 {
            /*
            literal
             */
            let mut lit;
            if lithist & 0xf != 0 {
                // if there was a non-ascii character in the four previous characters,
                // read in 9 bits and keep 8
                read_bits_count -= 9;
                lit = (read_bits >> read_bits_count & 0xff) as u8;
            } else {
                // skip 1 bit and get the next 7
                read_bits_count -= 8;
                lit = (read_bits >> read_bits_count & 0x7f) as u8;
                // if between 32 and 127, was plain ascii
                if (lit) < 32 {
                    if (lit) < 24 {
                        // control chars have two 0 bits prepended, shift in data bits
                        read_bits_count -= 2;
                        lit = ((lit) << 2) | (read_bits >> read_bits_count & 3) as u8;
                    } else {
                        // 8 bit values have three 0 bits prepended, shift in data bits
                        read_bits_count -= 3;
                        lit = ((lit) << 3) | (read_bits >> read_bits_count & 7) as u8;
                    }
                    // adjust for offset during encoding
                    lit -= 64;
                }
            }
            if current_dest_pos >= max_dest_pos {
                return Err(String::from("too much output"));
            }

            dst.push(lit);
            current_dest_pos += 1;
            // keep history of previously seen literals, 0 if ascii, 1 if not
            lithist = (lithist << 1) | if !(32..=127).contains(&lit) { 1 } else { 0 };
        } else {
            /*
            length
             */
            if len < 255 {
                read_bits_count -= LENBITS[len] as u32;
            } else {
                read_bits_count -= D_BIG_LEN_BITS;
                // rbc = 32 -> 26
                let mut code = ((read_bits >> read_bits_count & (((1) << D_BIG_LEN_BITS) - 1))
                    - D_BIG_LEN_CODE as usize) as u32;
                // code = top 6 bits - 111100
                // 00 -> 7, 01 -> {8, 9}, 10 -> {10..15}, 10 -> higher
                len = DMAX_FAST_LEN;
                let mut step = D_BIG_LEN_BASE;
                let mut shift = D_BIG_LEN_BITS & 1 ^ 1; // 1 if even, 0 if odd
                while code >= step {
                    if read_bits_count == 0 {
                        return Err(String::from("len out of range"));
                    }
                    len += step as usize;
                    code -= step;
                    code <<= 1;
                    read_bits_count -= 1;

                    // get another bit
                    code |= (read_bits >> read_bits_count & 1) as u32;
                    step <<= shift;
                    shift ^= 1;
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

            let s: usize = current_dest_pos - off;
            // can't use extend_from_within trivially, because the length can go past the current end of the
            // destination
            // it also appears to be slower than the byte by byte push when operating
            // in single elements at the end of dst.

            // off == 1 frequently happens, resize the vector and fill with that character
            if off == 1 {
                let byte = dst[s];
                dst.resize(current_dest_pos + len, byte);
            } else {
                // exponential extending from the offset is beneficial but not
                // as much a win over byte pushing as expected.
                // Pre-allocate the full target size
                dst.resize(current_dest_pos + len, 0);
                let fastlen = std::cmp::min(current_dest_pos - s, len);
                let mut boost = 0;
                while len >= fastlen + boost {
                    dst.copy_within(s..s + fastlen + boost, current_dest_pos);
                    len -= fastlen + boost;
                    current_dest_pos += fastlen + boost;
                    boost += fastlen + boost;
                }
                dst.copy_within(s..s + len, current_dest_pos);
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
