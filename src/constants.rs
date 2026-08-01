// Copyright 2024-2026 by Michael Stroucken
//pub const MAX_SEQ_START: u32 = 256;
//pub const MAX_SEQ_MASK: u32 = 8;
pub const MIN_DECODE: u32 = 8;
/// need at least a match of this size in the dictionary to make it
/// worth / possible storing a (length, offset) pair rather than literals
pub const MIN_MATCH: usize = 3;
/// bit mask for hash
pub const HASH_MASK: u32 = (1 << HASH_LOG) - 1;
//pub const HASH_SIZE: u32 = 16384;
/// size of hash in bits
pub const HASH_LOG: u32 = 14;
/// largest possible offset
pub const WHACK_MAX_OFF: u16 = 16384;
/// The smallest big len encoding is 9 bits long (7 bits traversal, 2 bits remainder)
pub const BIG_LEN_BITS: u32 = 9;
/// starting items to encode for big lens /
/// smallest encoding has space for two bits of remainder length
pub const BIG_LEN_BASE: u32 = 1 << 2;
/// minimum code for large length encoding /
/// starting point of the Huffman tree construction (7 bits traversal, 2 bits remainder)
pub const BIG_LEN_CODE: u16 = 0b111110100;

/// minimum length of an offset (3 bit bit count, 3 bit offset)
pub const MIN_OFF_BITS: u8 = 6;
pub const MAX_OFF_BITS: u8 = MIN_OFF_BITS + 8;

/// size of LENTAB
pub const MAX_FAST_LEN: usize = LENTAB.len();
/// max. length encodable in 24 bits
pub const MAXLEN: usize = 2051;

// decoding has a smaller predefined Huffman tree
// the tree is regular starting at 6, degenerate at 5
/// Number of starting 1 bits in Huffman tree signifying big lens
pub const D_BIG_LEN_BITS: u32 = 6;
/// starting items to encode for big lens
pub const D_BIG_LEN_BASE: u32 = 1;
/// minimum length to decode as big len
pub const DMAX_FAST_LEN: usize = LENBITS.len();
/// minimum code for large length encoding
pub const D_BIG_LEN_CODE: u8 = 0b111100;

/// decoding of the length value by traversing the tree
/// 0XXXX is a literal, use length = 0 to flag that
/// 10XXX is 3
/// 110XX is 4
/// 11100 is 5
/// 11101 is 6
/// 1111X is left for big len decoding
pub static LENVAL: [u8; 1 << (D_BIG_LEN_BITS - 1)] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 6, 255,
    255,
];
/// length of the tree to this length (only valid for index >= 3)
pub static LENBITS: [u8; 7] = [0, 0, 0, 2, 3, 5, 5];

/// looking up the first four bits of the offset symbol gives a base offset
pub static OFFBASE: [u16; 16] = [
    0, 0x20, 0x40, 0x60, 0x80, 0xc0, 0x100, 0x180, 0x200, 0x300, 0x400, 0x600, 0x800, 0xc00,
    0x1000, 0x2000,
];
/// count of bits to read for the normalized encoded offset
pub static OFFBITS: [u8; 16] = [5, 5, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 12, 13];

pub struct Huff {
    /// length of the code
    pub bits: u16,
    /// the code
    pub encode: usize,
}

pub static LENTAB: [Huff; 9] = [
    Huff {
        bits: 2,
        encode: 0b10,
    },
    Huff {
        bits: 3,
        encode: 0b110,
    },
    Huff {
        bits: 5,
        encode: 0b11100,
    },
    Huff {
        bits: 5,
        encode: 0b11101,
    },
    Huff {
        bits: 6,
        encode: 0b111100,
    },
    Huff {
        bits: 7,
        encode: 0b1111010,
    },
    Huff {
        bits: 7,
        encode: 0b1111011,
    },
    Huff {
        bits: 8,
        encode: 0b11111000,
    },
    Huff {
        bits: 8,
        encode: 0b11111001,
    },
];
