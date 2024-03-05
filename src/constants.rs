//pub const MAX_SEQ_START: u32 = 256;
//pub const MAX_SEQ_MASK: u32 = 8;
pub const MIN_DECODE: u32 = 8;
pub const MIN_MATCH: usize = 3;
pub const HASH_MASK: u32 = 16383;
//pub const HASH_SIZE: u32 = 16384;
pub const HASH_LOG: u32 = 14;
pub const WHACK_MAX_OFF: u16 = 16384;
pub const BIG_LEN_BITS: u32 = 9;
pub const BIG_LEN_BASE: u32 = 4; //starting items to encode for big lens
pub const BIG_LEN_CODE: u16 = 500; //minimum code for large length encoding
pub const MIN_OFF_BITS: u8 = 6;
pub const MAX_OFF_BITS: u8 = MIN_OFF_BITS + 8;
pub const MAX_FAST_LEN: u8 = 9;
pub const MAXLEN: usize = 2051; // max. length encodable in 24 bits

pub const D_BIG_LEN_BITS: u32 = 6;
pub const D_BIG_LEN_BASE: u32 = 1; //starting items to encode for big lens
pub const DMAX_FAST_LEN: usize = 7;
pub const D_BIG_LEN_CODE: u8 = 60; //minimum code for large length encoding

pub static LENVAL: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 6, 255,
    255,
];
pub static LENBITS: [u8; 7] = [0, 0, 0, 2, 3, 5, 5];
pub static OFFBITS: [u8; 16] = [5, 5, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 12, 13];
pub static OFFBASE: [u16; 16] = [
    0, 0x20, 0x40, 0x60, 0x80, 0xc0, 0x100, 0x180, 0x200, 0x300, 0x400, 0x600, 0x800, 0xc00,
    0x1000, 0x2000,
];

pub struct Huff {
    pub bits: u16,     // length of the code
    pub encode: usize, // the code
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
