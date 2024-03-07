//! This crate is a Rust port of Plan9's whack compression scheme as used
//! within the venti storage system. Original authors unknown, C source
//! came via Russ Cox and the 9fans/plan9port repository.
//!
//! Use the unwhack function to decompress, and whackblock to compress.
// Copyright 2024 by Michael Stroucken
mod constants;
mod testdata;
pub mod unwhack;
pub mod whack;

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose;
    use base64::Engine;

    use self::testdata::{large_compressed_data, large_uncompressed_data, random_data};

    use super::*;

    #[test]
    /// test if some dictionary compression happens
    pub fn whack_minimal() -> Result<(), String> {
        let src = b"foofoofoo".to_vec();
        let rv = whack::whackblock(&src, src.len());
        if rv.is_some() {
            //Err(String::from_utf8_lossy(&rv1.unwrap()).to_string())
            Ok(())
        } else {
            Err(String::from("did not compress"))
        }
    }

    #[test]
    /// test if compressed data uncompresses to data
    pub fn whack_test() -> Result<(), String> {
        let compressed = large_compressed_data();
        let decompressed = large_uncompressed_data();
        let src = general_purpose::STANDARD.decode(decompressed).unwrap();
        let rv = whack::whackblock(&src, src.len());
        if rv.is_some() {
            let target = general_purpose::STANDARD.decode(compressed).unwrap();

            let result = rv.unwrap();
            if target != result {
                return Err(String::from("compressed result doesn't match ground truth"));
            }
            Ok(())
        } else {
            Err(String::from("did not compress"))
        }
    }

    #[test]
    // test if compression of random data bails out if no compression achieved
    pub fn whack_random() -> Result<(), String> {
        let decompressed = random_data();
        let src = general_purpose::STANDARD.decode(decompressed).unwrap();
        let rv = whack::whackblock(&src, src.len());
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
    /// test if uncompressed data compresses to data
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
