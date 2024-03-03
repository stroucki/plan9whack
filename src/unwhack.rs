
pub const MAX_SEQ_START: u32 = 256;
pub const MAX_SEQ_MASK: u32 = 8;
pub const MIN_DECODE: u32 = 8;
pub const MIN_MATCH: u32 = 3;
pub const HASH_MASK: u32 = 16383;
pub const HASH_SIZE: u32 = 16384;
pub const HASH_LOG: u32 = 14;
pub const WHACK_MAX_OFF: u32 = 16384;
pub const WHACK_ERR_LEN: u32 = 64;
pub const WHACK_STATS: u32 = 8;
pub const D_BIG_LEN_BITS: u32 = 6;
pub const D_BIG_LEN_BASE: u32 = 1; //starting items to encode for big lens
pub const DMAX_FAST_LEN: usize = 7;
pub const D_BIG_LEN_CODE: u8 = 60; //minimum code for large length encoding

static LENVAL: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 6, 255,
    255,
];
static LENBITS: [u8; 7] = [0, 0, 0, 2, 3, 5, 5];
static OFFBITS: [u8; 16] = [5, 5, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 12, 13];
static OFFBASE: [u16; 16] = [
    0, 0x20, 0x40, 0x60, 0x80, 0xc0, 0x100, 0x180, 0x200, 0x300, 0x400, 0x600, 0x800, 0xc00,
    0x1000, 0x2000,
];

pub fn unwhackinit() {}

pub fn unwhack(src: &Vec<u8>, ndst: usize) -> Result<Vec<u8>, String> {
    let mut dst: Vec<u8> = Vec::new();
    let mut dpos = 0;
    let mut spos = 0;
    let mut uwnbits: u32 = 0;
    let mut uwbits: usize = 0;
    let mut overbits = 0;
    let smax = 0;
    let dmax = 0;
    let mut len: usize = 0;
    let mut lithist: usize = 0;
    let mut lit: u8 = 0;
    let mut code: u32 = 0;
    let mut use_0 = 0;
    let mut bits: u32 = 0;
    let mut off: usize = 0;

    while spos < smax || uwnbits - overbits >= MIN_DECODE {
        while uwnbits <= 24 {
            uwbits <<= 8;
            if spos < smax {
                let fresh0 = spos;
                spos += 1;
                uwbits |= src[spos] as usize;
            } else {
                overbits += 8;
            }
            uwnbits += 8;
        }
        /*
        literal
         */
        len = LENVAL[(uwbits >> uwnbits - 5 & 0x1f) as usize] as usize;
        if len == 0 {
            if lithist & 0xf != 0 {
                uwnbits -= 9;
                lit = (uwbits >> uwnbits & 0xff) as u8;
                lit = lit & 255;
            } else {
                uwnbits -= 8;
                lit = (uwbits >> uwnbits & 0x7f) as u8;
                if (lit) < 32 {
                    if (lit) < 24 {
                        uwnbits -= 2;
                        lit = ((lit) << 2) | (uwbits >> uwnbits & 3) as u8;
                    } else {
                        uwnbits -= 3;
                        lit = ((lit) << 3) | (uwbits >> uwnbits & 7) as u8;
                    }
                    lit = lit - 64;
                }
            }
            if dpos >= dmax {
                return Err(String::from("too much output"));
            }

            let fresh1 = dpos;
            dpos += 1;
            dst[fresh1] = lit;
            lithist = (lithist << 1) | if lit < 32 || lit > 127 { 1 } else { 0 };
        } else {
            /*
            length
             */
            if len < 255 {
                uwnbits -= LENBITS[len as usize] as u32;
            } else {
                uwnbits -= D_BIG_LEN_BITS;
                code = ((uwbits >> uwnbits & (((1) << D_BIG_LEN_BITS) - 1)) - D_BIG_LEN_CODE as usize)
                    as u32;
                len = DMAX_FAST_LEN;
                use_0 = D_BIG_LEN_BASE;
                bits = D_BIG_LEN_BITS & 1 ^ 1;
                while code >= use_0 {
                    len += use_0 as usize;
                    code -= use_0;
                    code <<= 1;
                    uwnbits -= 1;
                    if uwnbits < 0 {
                        return Err(String::from("len out of range"));
                    }

                    code |= (uwbits >> uwnbits & 1) as u32;
                    use_0 <<= bits;
                    bits ^= 1;
                }
                len += code as usize;
                while uwnbits <= 24 {
                    uwbits <<= 8;
                    if spos < smax {
                        let fresh2 = spos;
                        spos += 1;
                        uwbits |= src[fresh2] as usize;
                    } else {
                        overbits += 8;
                    }
                    uwnbits += 8;
                }
            }
            /*
            offset
             */
            uwnbits -= 4;
            bits = (uwbits >> uwnbits & 0xf) as u32;
            off = OFFBASE[bits as usize] as usize;
            bits = OFFBITS[bits as usize] as u32;
            uwnbits -= bits;
            off = off | (uwbits >> uwnbits) as usize & (((1) << bits) - 1);
            off += 1;
            if off > dpos {
                return Err(String::from(
                    "offset out of range: off={off} d={dpos} len={len} nbits={uwnbits}",
                ));
            }

            if dpos + len > dmax {
                return Err(String::from("len out of range"));
            }

            let mut s;
            let mut i;
            s = dpos - off;
            i = 0;
            while i < len {
                dst[dpos+i] = dst[s+i];
                i += 1;
            }
            dpos += len;
        }
    }
    if uwnbits < overbits {
        return Err(String::from("compressed data overrun"));
    }

    len = dpos;
    assert_eq!(len, ndst);
    Ok(dst)
}
