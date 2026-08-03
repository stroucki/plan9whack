//! This crate is a Rust port of Plan9's whack compression scheme as used
//! within the venti storage system. Original authors unknown, C source
//! came via Russ Cox and the 9fans/plan9port repository.
//!
//! Use the `unwhack` function to decompress, and `whackblock` to compress.
//! A `whack` function also exists if you want to control some parameters
//! of compression, or want to collect statistics.
//!
//! Internally, whack walks through the input by byte and tries to look up the
//! presence of a three byte set in a dictionary. If it was not previously
//! seen, a literal is emitted, where ASCII runs get favorable encoding, but
//! based on history can switch back to neutral binary encoding.
//!
//! If the trigraph was previously seen, depending on the Whack initialization
//! parameter, additional searches in the dictionary will be made until a
//! length minimum is satisfied, or there are no more matches. A (length,
//! offset) pair is emitted, where the length is a variable length encoded
//! integer between 3 and 2051, derived by a fixed Huffman tree
//! representation. The offset consists of a bit count, favoring lower
//! offsets, followed by the bits of the offset with the leading 1 bit
//! omitted.
//!
//! The trigraph at each point is hashed and is the key to a table pointing to
//! an offset in the history. The previous value at that hash is given to a
//! next entry keyed by the current dictionary position.
// Copyright 2024-2026 by Michael Stroucken
mod constants;
mod testdata;
pub mod unwhack;
pub mod whack;

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose;

    use crate::whack::whackinit;
    #[cfg(test)]
    use crate::whack::{Stats, Whack};

    use self::testdata::*;

    use super::*;

    use proptest::prelude::*;

    #[cfg(test)]
    /// compress src and compare to target, uncompress target and compare to src
    fn compress_decompress(w: &mut Whack, src: &[u8], target: &[u8]) -> Result<(), String> {
        let mut stats = Stats::default();
        let rv = whack::whack(w, &src, &mut stats);
        if rv.is_none() {
            return Err(String::from("did not compress"));
        }

        let result = rv.unwrap();

        if target != result {
            return Err(String::from("compressed result doesn't match ground truth"));
        }
        let x = target;
        let target = src;
        let src = x;
        let rv = unwhack::unwhack(&src, target.len());
        if rv.is_err() {
            return Err(rv.err().unwrap());
        }
        let result = rv.unwrap();
        if target != result {
            return Err(String::from(
                "uncompressed result does not match ground truth",
            ));
        }
        Ok(())
    }

    #[test]
    /// test if source too small to compress
    pub fn whack_onefoo() -> Result<(), String> {
        let src = b"foo".to_vec();
        let rv = whack::whackblock(&src);
        if rv.is_none() {
            //Err(String::from_utf8_lossy(&rv1.unwrap()).to_string())
            Ok(())
        } else {
            Err(String::from("should not have compressed"))
        }
    }

    #[test]
    /// test if some dictionary compression happens
    pub fn whack_threefoo() -> Result<(), String> {
        let src = b"foofoofoo".to_vec();
        let rv = whack::whackblock(&src);
        if rv.is_some() {
            //Err(String::from_utf8_lossy(&rv1.unwrap()).to_string())
            Ok(())
        } else {
            Err(String::from("did not compress"))
        }
    }

    #[test]
    /// test if compression of a large amount of 0 bits works
    pub fn whack_many0bits() -> Result<(), String> {
        let src = [0u8; 65536].to_vec();

        let target = general_purpose::STANDARD
            .decode(compressed_65k_0bits())
            .unwrap();

        let mut w = whackinit(6);
        compress_decompress(&mut w, &src, &target)
    }

    #[test]
    /// test if compression of a large amount of 1 bits works
    pub fn whack_many1bits() -> Result<(), String> {
        let src = [255u8; 65536].to_vec();

        let target = general_purpose::STANDARD
            .decode(compressed_65k_1bits())
            .unwrap();

        let mut w = whackinit(6);
        compress_decompress(&mut w, &src, &target)
    }

    #[test]
    /// test if compression of 0..512 works
    pub fn whack_countup() -> Result<(), String> {
        let mut src = Vec::new();
        for n in 0..512 {
            src.push(n as u8);
        }

        let target = general_purpose::STANDARD
            .decode(compressed_512_countup())
            .unwrap();

        let mut w = whackinit(6);
        compress_decompress(&mut w, &src, &target)
    }

    #[test]
    /// test if uncompressed data compresses to compressed data
    pub fn whack_test() -> Result<(), String> {
        let src = general_purpose::STANDARD
            .decode(large_uncompressed_data())
            .unwrap();
        let target = general_purpose::STANDARD
            .decode(large_compressed_data())
            .unwrap();
        let mut w = whackinit(6);
        compress_decompress(&mut w, &src, &target)
    }

    #[test]
    // test if compression of random data bails out if no compression achieved
    pub fn whack_random() -> Result<(), String> {
        let decompressed = random_data();
        let src = general_purpose::STANDARD.decode(decompressed).unwrap();
        let rv = whack::whackblock(&src);
        if rv.is_some() {
            if src.len() > rv.unwrap().len() {
                // should really be impossible
                return Err(String::from("result was expanded"));
            }
            Err(String::from("test data not incompressible enough"))
        } else {
            Ok(())
        }
    }

    #[test]
    /// test if uncompression of empty data works
    pub fn unwhack_null() -> Result<(), String> {
        let src = Vec::new();
        let rv = unwhack::unwhack(&src, src.len());
        if rv.is_ok() {
            Ok(())
        } else {
            Err(rv.err().unwrap())
        }
    }

    #[test]
    /// test if compressed data uncompresses to uncompressed data
    pub fn unwhack_test() -> Result<(), String> {
        let compressed = large_compressed_data();
        let decompressed = large_uncompressed_data();
        let src = general_purpose::STANDARD.decode(compressed).unwrap();
        let target = general_purpose::STANDARD.decode(decompressed).unwrap();
        let rv = unwhack::unwhack(&src, target.len());
        if rv.is_ok() {
            let result = rv.unwrap();
            if target != result {
                return Err(String::from(
                    "decompressed result doesn't match ground truth",
                ));
            }
            Ok(())
        } else {
            Err(rv.err().unwrap())
        }
    }

    #[test]
    /// test match extension that crosses u64 boundaries and both aligned and unaligned cases
    pub fn whack_word_boundary_match() -> Result<(), String> {
        // aligned pattern: 8-byte repeated sequence
        let mut data = Vec::new();
        for _ in 0..1024 {
            data.extend_from_slice(b"ABCDEFGH");
        }
        let compressed = whack::whackblock(&data).ok_or("did not compress aligned")?;
        let decompressed = unwhack::unwhack(&compressed, data.len()).map_err(|e| e)?;
        if decompressed != data {
            return Err(String::from("decompressed mismatch aligned"));
        }

        // unaligned pattern: prefix a single byte so matches are unaligned
        let mut data2 = Vec::new();
        data2.push(0u8);
        for _ in 0..1024 {
            data2.extend_from_slice(b"ABCDEFGH");
        }
        let compressed2 = whack::whackblock(&data2).ok_or("did not compress unaligned")?;
        let decompressed2 = unwhack::unwhack(&compressed2, data2.len()).map_err(|e| e)?;
        if decompressed2 != data2 {
            return Err(String::from("decompressed mismatch unaligned"));
        }

        Ok(())
    }

    fn test_data() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            // Random (mostly incompressible)
            proptest::collection::vec(any::<u8>(), 0..10000),
            // Highly compressible
            (any::<u8>(), 0usize..10000).prop_map(|(b, len)| vec![b; len]),
            // Limited alphabet
            proptest::collection::vec(0u8..4u8, 0..10000),
            // Repeated patterns
            (0u8..255u8, 1usize..128, 1usize..100).prop_map(|(byte, pattern_len, repeats)| {
                vec![byte; pattern_len].repeat(repeats)
            }),
        ]
    }

    proptest! {
        #[test]
        fn roundtrip(data in test_data()) {
            let compressed = whack::whackblock(&data);
            if compressed.is_some() {
                let compressed = compressed.unwrap();
                let decompressed = unwhack::unwhack(&compressed, data.len()).expect("decompression failed");
                prop_assert_eq!(&decompressed, &data);
                /*
                let dlen = data.len();
                let clen = compressed.len();
                let pct = clen*100/dlen;
                println!("Compressed size: {} Uncompressed size: {} %: {}", compressed.len(), data.len(), pct);
                */
                return Ok(());
            }
            //println!("data size {} did not compress",data.len());
        }
    }
}
