//! This crate is a Rust port of Plan9's whack compression scheme as used
//! within the venti storage system. Original authors unknown, C source
//! came via Russ Cox and the 9fans/plan9port repository.
//!
//! Use the `unwhack` function to decompress, and `whackblock` to compress.
//! A `whack` function also exists if you want to control some parameters
//! of compression, or want to collect statistics.
// Copyright 2024-2026 by Michael Stroucken
mod constants;
mod testdata;
pub mod unwhack;
pub mod whack;

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose;

    use self::testdata::*;

    use super::*;

    #[cfg(test)]
    fn compress_decompress(src: &[u8], target: &[u8]) -> Result<(), String> {
        let rv = whack::whackblock(&src);
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
        let t1 = b"foo".to_vec();
        let r1 = whack::whackblock(&t1);
        println!("r1: {:?}", r1);
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

        return compress_decompress(&src, &target);
    }

    #[test]
    /// test if compression of a large amount of 0 bits works
    pub fn whack_many1bits() -> Result<(), String> {
        let src = [255u8; 65536].to_vec();

        let target = general_purpose::STANDARD
            .decode(compressed_65k_1bits())
            .unwrap();

        compress_decompress(&src, &target)
    }

    #[test]
    /// test if compression of a large amount of 0 bits works
    pub fn whack_countup() -> Result<(), String> {
        let mut src = Vec::new();
        for n in 0..512 {
            src.push(n as u8);
        }

        let target = general_purpose::STANDARD
            .decode(compressed_512_countup())
            .unwrap();

        return compress_decompress(&src, &target);
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
        compress_decompress(&src, &target)
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
            Err(String::from("test data not uncompressible enough"))
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
}
