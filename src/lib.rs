mod constants;
mod testdata;
pub mod unwhack;
pub mod whack;

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose;

    use self::testdata::{large_compressed_data, large_uncompressed_data};

    use super::*;

    #[test]
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
    pub fn whack_test() -> Result<(), String> {
        let compressed = large_compressed_data();
        let decompressed = large_uncompressed_data();
        let src = general_purpose::STANDARD.decode(decompressed).unwrap();
        let rv = whack::whackblock(&src, 10442);
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
    pub fn unwhack_null() -> Result<(), String> {
        let src = Vec::new();
        let rv = unwhack::unwhack(&src, 0);
        if rv.is_ok() {
            Ok(())
        } else {
            Err(rv.err().unwrap())
        }
    }

    #[test]
    pub fn unwhack_test() -> Result<(), String> {
        let compressed = large_compressed_data();
        let decompressed = large_uncompressed_data();
        let src = general_purpose::STANDARD.decode(compressed).unwrap();
        let rv = unwhack::unwhack(&src, 10442);
        if rv.is_ok() {
            let target = general_purpose::STANDARD.decode(decompressed).unwrap();

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
