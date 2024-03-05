use crate::constants::*;

pub fn unwhackinit() {}

pub fn unwhack(src: &Vec<u8>, ndst: usize) -> Result<Vec<u8>, String> {
    let mut dst: Vec<u8> = Vec::with_capacity(ndst);
    let mut dpos = 0;
    let mut spos = 0;
    let mut uwnbits: u32 = 0;
    let mut uwbits: usize = 0;
    let mut overbits = 0;
    let smax = src.len();
    let dmax = ndst;
    let mut len: usize;
    let mut lithist: usize = !0;
    let mut lit: u8;
    let mut code: u32;
    let mut use_0;
    let mut bits: u32;
    let mut off: usize;

    while spos < smax || uwnbits - overbits >= MIN_DECODE {
        while uwnbits <= 24 {
            uwbits <<= 8;
            if spos < smax {
                uwbits |= src[spos] as usize;
                spos += 1;
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

            dst.push(lit);
            dpos += 1;
            lithist = (lithist << 1) | if lit < 32 || lit > 127 { 1 } else { 0 };
        } else {
            /*
            length
             */
            if len < 255 {
                uwnbits -= LENBITS[len as usize] as u32;
            } else {
                uwnbits -= D_BIG_LEN_BITS;
                code = ((uwbits >> uwnbits & (((1) << D_BIG_LEN_BITS) - 1))
                    - D_BIG_LEN_CODE as usize) as u32;
                len = DMAX_FAST_LEN;
                use_0 = D_BIG_LEN_BASE;
                bits = D_BIG_LEN_BITS & 1 ^ 1;
                while code >= use_0 {
                    if uwnbits == 0 {
                        return Err(String::from("len out of range"));
                    }
                    len += use_0 as usize;
                    code -= use_0;
                    code <<= 1;
                    uwnbits -= 1;

                    code |= (uwbits >> uwnbits & 1) as u32;
                    use_0 <<= bits;
                    bits ^= 1;
                }
                len += code as usize;
                while uwnbits <= 24 {
                    uwbits <<= 8;
                    if spos < smax {
                        uwbits |= src[spos] as usize;
                        spos += 1;
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
            off |= (uwbits >> uwnbits) as usize & (((1) << bits) - 1);
            off += 1;
            if off > dpos {
                return Err(format!(
                    "offset out of range: off={off} d={dpos} len={len} nbits={uwnbits}",
                ));
            }

            if dpos + len > dmax {
                return Err(String::from("len out of range"));
            }

            let s = dpos - off;
            let mut i = 0;
            while i < len {
                dst.push(dst[s + i]);
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
