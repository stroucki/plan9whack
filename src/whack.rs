// Copyright 2024-2026 by Michael Stroucken
use crate::constants::*;
use std::ptr;

/// Compression dictionary
///
/// It maintains the dictionary indices by implementing a
/// hash table of 16384 entries that point to a previous
/// entry at that position. Values in the arrays are
/// u16 starting at values that cannot be considered
/// hash table indices, if the two tables used to be combined.
pub struct Whack {
    /// beginning dictionary index
    pub begin: u16,
    /// lookup value from hash to current dictionary entry
    /// index ranges from 0..16384, but values can overflow
    pub hash: [u16; 1 << HASH_LOG],
    /// this is actually the previous dictionary entry for a dictionary entry
    pub next: [u16; WHACK_MAX_OFF as usize],
    /// maximum length to consider
    pub thwmaxcheck: u32,
}

/// Collect status from compression
#[derive(Default)]
pub struct Stats {
    pub statbytes: usize,
    pub statoutbytes: usize,
    pub statlits: usize,
    pub statmatches: usize,
    pub statlitbits: usize,
    pub statoffbits: usize,
    pub statlenbits: usize,
}

struct DictLookup {
    pub len: u16,
    pub off: u16,
}

/// Create a compressor state object
pub fn whackinit(level: u8) -> Whack {
    let mut thwmaxcheck;
    thwmaxcheck = (1) << level;
    thwmaxcheck -= thwmaxcheck >> 2;
    // thwmaxcheck = 0.75 * 2^level
    thwmaxcheck = thwmaxcheck.clamp(2, 1024);

    Whack {
        begin: 2 * WHACK_MAX_OFF, // values out of range are used to indicate invalid dictionary entries
        hash: [0; 16384],
        next: [0; 16384],
        thwmaxcheck,
    }
}

/// find a string in the dictionary
fn whackmatch(
    w: &Whack,
    src: &[u8],
    current_source_position: usize,
    max_source_position: usize,
    hash: u16,
    current_dict_position: u16,
) -> Option<DictLookup> {
    let mut last_dict_position: u16;
    let mut candidate_offset: u16;
    let mut bestoff: u16;
    let mut bestlen: usize;
    let mut check: u32;
    let mut current_match_position: usize = current_source_position;
    let mut candidate_match_position: usize;
    let mut last_candidate_offset: u16;
    let mut max_match_position = max_source_position;

    if max_match_position < current_match_position + MIN_MATCH {
        return None;
    }

    if current_match_position + MAXLEN < max_match_position {
        max_match_position = current_match_position + MAXLEN;
    }
    bestoff = 0;
    bestlen = 0;
    check = w.thwmaxcheck;
    last_candidate_offset = 0;
    last_dict_position = w.hash[hash as usize];
    loop {
        if check == 0 {
            break;
        }
        check -= 1;

        candidate_offset = current_dict_position.wrapping_sub(last_dict_position);
        if candidate_offset <= last_candidate_offset || candidate_offset > WHACK_MAX_OFF {
            break;
        }

        /*
         * don't need to check for the end because
         * 1) s too close check above
         */

        candidate_match_position = current_match_position - candidate_offset as usize;
        if src[current_match_position] == src[candidate_match_position]
            && src[current_match_position + 1] == src[candidate_match_position + 1]
            && src[current_match_position + 2] == src[candidate_match_position + 2]
            && (bestlen == 0
                || max_match_position - current_match_position > bestlen
                    && src[current_match_position + bestlen]
                        == src[candidate_match_position + bestlen])
        {
            candidate_match_position += 3;
            current_match_position += 3;

            // Fast extension: compare u64 words where possible for speed, then finish byte-by-byte.
            while current_match_position + 8 <= max_match_position
                && candidate_match_position + 8 <= max_match_position
            {
                let (a, b);
                // SAFETY: we checked that there are still at least 8 bytes available
                unsafe {
                    a = ptr::read_unaligned(src.as_ptr().add(current_match_position) as *const u64);
                    b = ptr::read_unaligned(
                        src.as_ptr().add(candidate_match_position) as *const u64
                    );
                }

                if a == b {
                    current_match_position += 8;
                    candidate_match_position += 8;
                    continue;
                }

                // this looks like a small optimization, but resulted in a 10% improvement
                // with the limited-vocab benchmark
                let diff = a ^ b;
                let byte_index = if cfg!(target_endian = "little") {
                    (diff.trailing_zeros() / 8) as usize
                } else {
                    (diff.leading_zeros() / 8) as usize
                };
                current_match_position += byte_index;
                // advance candidate position as well
                candidate_match_position += byte_index;

                break;
            }

            while current_match_position < max_match_position {
                if src[current_match_position] != src[candidate_match_position] {
                    break;
                }
                candidate_match_position += 1;
                current_match_position += 1;
            }
            if current_match_position - current_source_position > bestlen {
                bestlen = current_match_position - current_source_position;
                bestoff = candidate_offset;

                // it uses thwmaxcheck as the limit to the number of dictionary
                // checks and the best match length?
                if bestlen > w.thwmaxcheck as usize {
                    break;
                }
            }
        }
        current_match_position = current_source_position;
        last_candidate_offset = candidate_offset;
        last_dict_position = w.next[(last_dict_position & (WHACK_MAX_OFF - 1)) as usize];
    }

    Some(DictLookup {
        len: bestlen as u16,
        off: bestoff,
    })
}

/*
 * knuth vol. 3 multiplicative hashing
 * each byte x chosen according to rules
 * 1/4 < x < 3/10, 1/3 x < < 3/7, 4/7 < x < 2/3, 7/10 < x < 3/4
 * with reasonable spread between the bytes & their complements
 *
 * the 3 byte value appears to be as almost good as the 4 byte value,
 * and might be faster on some machines
 */
/*
#define hashit(c)       ((((ulong)(c) * 0x6b43a9) >> (24 - HashLog)) & HashMask)
*/
/// hash the bottom 24 bits of `c` into a 14 bit value
#[inline]
fn hashit(c: usize) -> u16 {
    ((((c & 0xffffff) * 0x6b43a9b5) >> (32 - HASH_LOG)) as u32 & HASH_MASK) as u16
}

/// Compress a section of data
///
/// lz77 compression with single lookup in a hash table for each block
///
/// Takes data in `src` and outputs a [`Vec<u8>`], updating
/// [`Stats`] in `stats`
///
/// # Errors
///
/// If source is too small, compressed data is larger than
/// source or likely to be so
pub fn whack(w: &mut Whack, src: &[u8], stats: &mut Stats) -> Option<Vec<u8>> {
    let mut current_source_position: usize;
    let mut target_source_position: usize;

    let mut half: usize;
    let mut current_output_length: usize;

    // these two need to be at least 32 bit
    let mut cont: usize; // lookahead of the next MIN_MATCH bytes
    let mut pending_output_bits: usize;

    let mut current_dict_position: u16;
    let mut lithist: u32;
    let mut pending_output_bits_length: u16;

    // for stats
    let mut lits: usize;
    let mut matches: usize;
    let mut offbits: usize;
    let mut lenbits: usize;

    let max_source_position = src.len();
    if max_source_position < MIN_MATCH {
        return None;
    }

    let mut dst = Vec::with_capacity(max_source_position);
    current_output_length = 0;
    let max_output_length: usize = max_source_position;
    current_dict_position = w.begin;
    current_source_position = 0;

    half = max_source_position >> 1;
    pending_output_bits_length = 0;
    pending_output_bits = 0;
    lits = 0;
    matches = 0;
    offbits = 0;
    lenbits = 0;
    lithist = !(0);

    // read in a match candidate
    cont = (((src[current_source_position] as u32) << 16)
        | ((src[current_source_position + 1] as u32) << 8)
        | (src[current_source_position + 2] as u32)) as usize;

    while current_source_position < max_source_position {
        let mut hash = hashit(cont);
        let mut match_len;
        let mut match_offset;
        let wmr = whackmatch(
            w,
            src,
            current_source_position,
            max_source_position,
            hash,
            current_dict_position,
        );
        if let Some(foo) = wmr {
            (match_offset, match_len) = (foo.off, foo.len);
        } else {
            (match_offset, match_len) = (0, 0);
        }
        target_source_position = current_source_position + match_len as usize;

        // flush pending bytes
        while pending_output_bits_length >= 8 {
            if current_output_length >= max_output_length {
                // fail if output length exceeds or equals source length
                w.begin = current_dict_position;
                return None;
            }
            let value = (pending_output_bits >> (pending_output_bits_length - 8)) as u8;
            dst.push(value);
            current_output_length += 1;
            pending_output_bits_length -= 8;
        }

        // act on the dictionary lookup
        if (match_len as usize) < MIN_MATCH {
            // output literal
            let mut current_byte = src[current_source_position] as u16;
            // append 1 if current byte is ASCII, else 0
            lithist = lithist << 1
                | if !(32..=127).contains(&current_byte) {
                    1
                } else {
                    0
                };

            if lithist & 0x1e != 0 {
                // if previously any of the last 4 characters were not ASCII
                // append byte extended by leading 0 bit
                pending_output_bits = pending_output_bits << 9 | current_byte as usize;
                pending_output_bits_length += 9;
            } else if lithist & 1 != 0 {
                // if the current character was not ASCII, add 64
                current_byte = (current_byte + 64) & 0xff;
                if current_byte < 96 {
                    // if current character was < 32
                    // append new byte extended by two leading 0 bits
                    pending_output_bits = pending_output_bits << 10 | current_byte as usize;
                    pending_output_bits_length += 10;
                } else {
                    // append new byte extended by three leading 0 bits
                    pending_output_bits = pending_output_bits << 11 | current_byte as usize;
                    pending_output_bits_length += 11;
                }
            } else {
                // if all of the last 5 characters were ASCII
                // append byte
                pending_output_bits = pending_output_bits << 8 | current_byte as usize;
                pending_output_bits_length += 8;
            }
            lits += 1;

            /*
             * speed hack
             * check for compression progress, bail if none achieved by halfway point
             */
            if current_source_position > half {
                // while longer runs of ascii literals are directly inserted into the output,
                // each literal symbol can have up to 3 0 bits prepended
                if (4 * current_source_position) < (5 * lits) {
                    // current_source_position < 1.25 lits
                    w.begin = current_dict_position;
                    return None;
                }
                half = max_source_position;
            }
            if current_source_position + MIN_MATCH <= max_source_position {
                w.next[(current_dict_position & (WHACK_MAX_OFF - 1)) as usize] =
                    w.hash[hash as usize];
                w.hash[hash as usize] = current_dict_position;
                if current_source_position + MIN_MATCH < max_source_position {
                    cont = cont << 8 | src[current_source_position + MIN_MATCH] as usize;
                }
            }
            current_dict_position = current_dict_position.wrapping_add(1);
            current_source_position += 1;
        } else {
            matches += 1;
            if (match_len as usize) > MAXLEN {
                match_len = MAXLEN as u16;
                target_source_position = current_source_position + match_len as usize;
            }
            // if we are doing (len, off) pair, len is definitely at least MIN_MATCH
            match_len -= MIN_MATCH as u16;

            if match_len < MAX_FAST_LEN as u16 {
                // there is a premade Huffman tree to encode (adjusted) match_len values from 0 to 9
                let huff = &LENTAB[match_len as usize];
                let bits = huff.bits;
                pending_output_bits = pending_output_bits << bits | huff.encode;
                pending_output_bits_length += bits;
                lenbits += bits as usize;
            } else {
                // for larger values, we deterministically add to the Huffman tree
                match_len -= MAX_FAST_LEN as u16;
                // match_len has now been reduced by 12
                let mut code = BIG_LEN_CODE as usize;
                let mut bits = BIG_LEN_BITS as u16;
                let mut step = BIG_LEN_BASE; // largest remainder + 1
                // we start at 5x1 01 xx
                // then go to 6x1 00 xx
                // then go to 6x1 01 xxx
                // then go to 7x1 00 xxx
                // then go to 7x1 01 xxxx
                // then go to 8x1 00 xxxx
                while match_len as u32 >= step {
                    match_len -= step as u16;
                    // the adding of the excess remainder and the left shift cause the 1s
                    // to aggregate and the branch to switch from 01 to 00
                    code = (code + step as usize) << 1;
                    // remainder bit size increases here when number of code bits is even
                    step <<= bits & 1 ^ 1;
                    bits += 1;
                }
                // 5    11100 x x 3x1 00 x
                // 6    11101 0 x 3x1 01 0
                // 7    111100 0 x 4x1 00 0
                // 8    1111010 1 x 4x1 01 1
                // 10   11111000 1 x 5x1 00 1
                // working backward above ^^^
                // 12   111110100 2 0 5x1 01 2;
                // 15   111110111
                // 16   1111110000 2 4 6x1 00 2
                // 19   1111110011
                // 20   11111101000 3 8 6x1 01 3
                // 27   11111101111
                // 28   111111100000 3 16 7x1 00 3
                // 35   111111100111
                // 36   1111111010000 4 24 7x1 01 4
                // 43   1111111010111
                // 44   1111111011000
                // 51   1111111011111
                // 52   11111111000000 4 40 8x1 00 4
                // 59   11111111000111
                // 60   11111111001000
                // 67   11111111001111
                // 68   111111110100000 5 56 8x1 01 5
                // 99   111111110111111
                // 100  1111111110000000 5 88 9x1 00 5
                // 131  1111111110011111
                // 132  11111111101000000 6 120 9x1 01 6
                // 2051 111111111111100111111111 9
                // 2052 1111111111111010000000000 invalid
                pending_output_bits = pending_output_bits << bits | (code + match_len as usize);
                pending_output_bits_length += bits;
                lenbits += bits as usize;

                // flush pending bytes
                while pending_output_bits_length >= 8 {
                    if current_output_length >= max_output_length {
                        // fail if output length exceeds source length
                        w.begin = current_dict_position;
                        return None;
                    }
                    dst.push((pending_output_bits >> (pending_output_bits_length - 8)) as u8);
                    current_output_length += 1;
                    pending_output_bits_length -= 8;
                }
            }

            /*
             * offset in history
             */
            // maximum for match_offset is WHACK_MAX_OFF
            match_offset -= 1;
            // find how many bits match_offset fits into
            let mut bits = MIN_OFF_BITS as u16;
            while match_offset >= (1) << bits {
                bits += 1;
            }
            if bits < (MAX_OFF_BITS - 1) as u16 {
                // add 3 bit bit count
                pending_output_bits =
                    pending_output_bits << 3 | (bits - MIN_OFF_BITS as u16) as usize;
                if bits != MIN_OFF_BITS as u16 {
                    bits -= 1;
                }
                pending_output_bits_length += bits + 3;
                offbits += bits as usize + 3;
            } else {
                // add 4 bit 1110 for 13 bits offset and 1111 for 14 bits offset
                pending_output_bits =
                    pending_output_bits << 4 | 0xe | (bits - (MAX_OFF_BITS - 1) as u16) as usize;
                bits -= 1;
                pending_output_bits_length += bits + 4;
                offbits += bits as usize + 4;
            }
            // unless match_offset fit into 6 bits, number of bits has been reduced by one
            // 63 would be encoded as RPN 6 << 63 63 & |
            // 64 would be encoded as RPN 6 << 64 63 & |
            // normalization because MSB will always be 1
            pending_output_bits =
                pending_output_bits << bits | (match_offset & (((1) << bits) - 1)) as usize;

            // walk down the match and update indices
            while current_source_position != target_source_position {
                if current_source_position + MIN_MATCH <= max_source_position {
                    hash = hashit(cont);
                    w.next[(current_dict_position & (WHACK_MAX_OFF - 1)) as usize] =
                        w.hash[hash as usize];
                    w.hash[hash as usize] = current_dict_position;
                    if current_source_position + MIN_MATCH < max_source_position {
                        cont = cont << 8 | src[current_source_position + MIN_MATCH] as usize;
                    }
                }
                current_dict_position = current_dict_position.wrapping_add(1);
                current_source_position += 1;
            }
        }
    }
    w.begin = current_dict_position;
    stats.statbytes += max_source_position;
    stats.statlits += lits;
    stats.statmatches += matches;
    stats.statlitbits +=
        current_output_length * 8 + pending_output_bits_length as usize - offbits - lenbits;
    /*
        // XXXstroucki that -2 can cause the value to become negative.
        // Original C source returns overflowed nonsense.
    stats.statlitbits += (current_output_length - 2) * 8 + pending_output_bits_length as usize
        - offbits as usize
        - lenbits as usize;
        */
    stats.statoffbits += offbits;
    stats.statlenbits += lenbits;

    // append 0 bits to finish out a byte
    if pending_output_bits_length & 7 != 0 {
        pending_output_bits <<= 8 - (pending_output_bits_length & 7);
        pending_output_bits_length += 8 - (pending_output_bits_length & 7);
    }
    while pending_output_bits_length >= 8 {
        // fail if output length exceeds source length
        if current_output_length >= max_output_length {
            return None;
        }
        dst.push((pending_output_bits >> (pending_output_bits_length - 8)) as u8);
        current_output_length += 1;
        pending_output_bits_length -= 8;
    }

    stats.statoutbytes += current_output_length;
    //assert_eq!(wdst, dst.len());
    Some(dst)
}

/// Compress a section of data
///
/// Takes data in `src` and returns an [`Option<Vec<u8>>`]
///
/// If source is too small, compressed data is larger than
/// source or likely to be so, the return value is None.
pub fn whackblock(src: &[u8]) -> Option<Vec<u8>> {
    let mut stats = Stats::default();
    let mut w = whackinit(6);
    whack(&mut w, src, &mut stats)
}
