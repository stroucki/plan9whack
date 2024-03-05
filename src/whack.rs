use crate::constants::*;

pub struct Whack {
    pub begin: u16,
    pub hash: [u16; 16384],
    pub next: [u16; 16384],
    pub thwmaxcheck: u32,
}

pub struct Stats {
    pub statbytes: usize,
    pub statoutbytes: usize,
    pub statlits: usize,
    pub statmatches: usize,
    pub statlitbits: usize,
    pub statoffbits: usize,
    pub statlenbits: usize,
}

pub struct DictLookup {
    pub len: u16,
    pub off: u16,
}

pub fn whackinit(level: u8) -> Whack {
    let mut thwmaxcheck;
    thwmaxcheck = (1) << level;
    thwmaxcheck -= thwmaxcheck >> 2;
    if thwmaxcheck < 2 {
        thwmaxcheck = 2;
    } else if thwmaxcheck > 1024 {
        thwmaxcheck = 1024;
    }
    Whack {
        begin: 2 * WHACK_MAX_OFF,
        hash: [0; 16384],
        next: [0; 16384],
        thwmaxcheck,
    }
}

/*
find a string in the dictionary
 */
pub fn whackmatch(
    b: &Whack,
    src: &[u8],
    ss: usize,
    mut esrc: usize,
    h: u16,
    now: u16,
) -> Option<DictLookup> {
    let mut then: u16;
    let mut off: u16;
    let mut bestoff: u16;
    let mut bestlen: usize;
    let mut check: u32;
    let mut s: usize = ss;
    let mut t: usize;
    let mut last: u16;

    if esrc < s + MIN_MATCH {
        return None;
    }

    if s + MAXLEN < esrc {
        esrc = s + MAXLEN;
    }
    bestoff = 0;
    bestlen = 0;
    check = b.thwmaxcheck;
    last = 0;
    then = b.hash[h as usize];
    loop {
        if !(check > 0) {
            break;
        }
        check = check - 1;

        off = now - then;
        if off <= last || off > WHACK_MAX_OFF {
            break;
        }

        /*
         * don't need to check for the end because
         * 1) s too close check above
         */

        t = s - off as usize;
        if src[s] == src[t] && src[s + 1] == src[t + 1] && src[s + 2] == src[t + 2] {
            if bestlen == 0 || esrc - s > bestlen && src[s + bestlen] == src[t + bestlen] {
                t += 3;
                s += 3;
                while s < esrc {
                    if src[s] != src[t] {
                        break;
                    }
                    t += 1;
                    s += 1;
                }
                if s - ss > bestlen {
                    bestlen = s - ss;
                    bestoff = off;
                    if bestlen > b.thwmaxcheck as usize {
                        break;
                    }
                }
            }
        }
        s = ss;
        last = off;
        then = b.next[(then & WHACK_MAX_OFF - 1) as usize];
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
pub fn hashit(c: usize) -> u16 {
    ((((c & 0xffffff) * 0x6b43a9b5) >> (32 - HASH_LOG)) as u32 & HASH_MASK) as u16
}

/*
 * lz77 compression with single lookup in a hash table for each block
 */
pub fn whack(w: &mut Whack, src: &[u8], n: usize, stats: &mut Stats) -> Option<Vec<u8>> {
    let mut s: usize;
    let mut ss: usize;
    let mut sss: usize;
    let esrc: usize;
    let mut half: usize;
    let mut wdst: usize;
    let wdmax: usize;
    let mut cont: usize;
    let mut code: usize;
    let mut wbits: usize;
    let mut now: u16;
    let mut toff: u16;
    let mut lithist: u32;
    let mut h: u16;
    let mut len: u16;
    let mut bits: u16;
    let mut use_0: u32;
    let mut wnbits: u16;
    let mut lits: usize;
    let mut matches: usize;
    let mut offbits: u16;
    let mut lenbits: u16;
    if n < MIN_MATCH {
        return None;
    }

    let mut dst = Vec::with_capacity(n);
    wdst = 0;
    wdmax = n;
    now = w.begin;
    s = 0;

    cont =
        (((src[s + 0] as u32) << 16) | ((src[s + 1] as u32) << 8) | (src[s + 2] as u32)) as usize;
    esrc = s + n;
    half = s + (n >> 1);
    wnbits = 0;
    wbits = 0;
    lits = 0;
    matches = 0;
    offbits = 0;
    lenbits = 0;
    lithist = !(0);
    while s < esrc {
        h = hashit(cont);

        sss = s;
        let wmr = whackmatch(w, &src, sss, esrc, h, now);
        if wmr.is_some() {
            let foo = wmr.unwrap();
            (toff, len) = (foo.off, foo.len);
        } else {
            (len, toff) = (0, 0);
        }
        ss = s + len as usize;

        while wnbits >= 8 {
            if wdst >= wdmax {
                w.begin = now;
                return None;
            }
            let value = (wbits >> wnbits - 8) as u8;
            dst.push(value);
            wdst += 1;
            wnbits -= 8;
        }
        if (len as usize) < MIN_MATCH {
            toff = src[s] as u16;
            lithist = lithist << 1 | if toff < 32 || toff > 127 { 1 } else { 0 };

            if lithist & 0x1e != 0 {
                wbits = wbits << 9 | toff as usize;
                wnbits += 9;
            } else if lithist & 1 != 0 {
                toff = toff + 64 & 0xff;
                if toff < 96 {
                    wbits = wbits << 10 | toff as usize;
                    wnbits += 10;
                } else {
                    wbits = wbits << 11 | toff as usize;
                    wnbits += 11;
                }
            } else {
                wbits = wbits << 8 | toff as usize;
                wnbits += 8;
            }
            lits += 1;

            /*
             * speed hack
             * check for compression progress, bail if none achieved
             */
            if s > half {
                if (4 * s) < (5 * lits) {
                    w.begin = now;
                    return None;
                }
                half = esrc;
            }
            if s + MIN_MATCH <= esrc {
                w.next[(now & (WHACK_MAX_OFF - 1) as u16) as usize] = w.hash[h as usize];
                w.hash[h as usize] = now;
                if s + MIN_MATCH < esrc {
                    cont = cont << 8 | src[s + MIN_MATCH] as usize;
                }
            }
            now += 1;
            s += 1;
        } else {
            matches += 1;
            if (len as usize) > MAXLEN {
                len = MAXLEN as u16;
                ss = s + len as usize;
            }
            len -= MIN_MATCH as u16;
            if len < MAX_FAST_LEN as u16 {
                bits = LENTAB[len as usize].bits;
                wbits = wbits << bits | LENTAB[len as usize].encode;
                wnbits += bits;
                lenbits += bits;
            } else {
                //stop();
                code = BIG_LEN_CODE as usize;
                bits = BIG_LEN_BITS as u16;
                use_0 = BIG_LEN_BASE;
                len -= MAX_FAST_LEN as u16;
                while len as u32 >= use_0 {
                    len -= use_0 as u16;
                    code = (code + use_0 as usize) << 1;
                    use_0 <<= bits & 1 ^ 1;
                    bits += 1;
                }
                wbits = wbits << bits | (code + len as usize);
                wnbits += bits;
                lenbits += bits;
                while wnbits >= 8 {
                    if wdst >= wdmax {
                        w.begin = now;
                        return None;
                    }
                    dst.push((wbits >> wnbits - 8) as u8);
                    wdst += 1;
                    wnbits -= 8;
                }
            }

            /*
             * offset in history
             */
            toff -= 1;
            bits = MIN_OFF_BITS as u16;
            while toff >= (1) << bits {
                bits += 1;
            }
            if bits < (MAX_OFF_BITS - 1) as u16 {
                wbits = wbits << 3 | (bits - MIN_OFF_BITS as u16) as usize;
                if bits != MIN_OFF_BITS as u16 {
                    bits -= 1;
                }
                wnbits += bits + 3;
                offbits += bits + 3;
            } else {
                wbits = wbits << 4 | 0xe | (bits - (MAX_OFF_BITS - 1) as u16) as usize;
                bits -= 1;
                wnbits += bits + 4;
                offbits += bits + 4;
            }
            wbits = wbits << bits | (toff & ((1) << bits) - 1) as usize;
            while s != ss {
                if s + MIN_MATCH <= esrc {
                    h = hashit(cont);
                    w.next[(now & (WHACK_MAX_OFF - 1) as u16) as usize] = w.hash[h as usize];
                    w.hash[h as usize] = now;
                    if s + MIN_MATCH < esrc {
                        cont = cont << 8 | src[s + MIN_MATCH] as usize;
                    }
                }
                now += 1;
                s += 1;
            }
        }
    }
    w.begin = now;
    stats.statbytes += esrc;
    stats.statlits += lits;
    stats.statmatches += matches;
    stats.statlitbits += (wdst - 2) * 8 + wnbits as usize - offbits as usize - lenbits as usize;
    stats.statoffbits += offbits as usize;
    stats.statlenbits += lenbits as usize;

    if wnbits & 7 != 0 {
        wbits <<= 8 - (wnbits & 7);
        wnbits += 8 - (wnbits & 7);
    }
    while wnbits >= 8 {
        if wdst >= wdmax {
            return None;
        }
        dst.push((wbits >> wnbits - 8) as u8);
        wdst += 1;
        wnbits -= 8;
    }

    stats.statoutbytes += wdst;
    assert_eq!(wdst, dst.len());
    return Some(dst);
}

pub fn whackblock(src: &[u8], ssize: usize) -> Option<Vec<u8>> {
    let mut stats = Stats {
        statbytes: 0,
        statoutbytes: 0,
        statlits: 0,
        statmatches: 0,
        statlitbits: 0,
        statoffbits: 0,
        statlenbits: 0,
    };
    let mut w = whackinit(6);
    whack(&mut w, &src, ssize, &mut stats)
}
