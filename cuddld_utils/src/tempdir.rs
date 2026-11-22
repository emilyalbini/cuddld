use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::path::PathBuf;

pub fn create_temp_dir() -> Result<PathBuf, Error> {
    for _ in 0..10 {
        let path = std::env::temp_dir().join(random_name()?);
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }

    Err(Error::other("all attempts to create a directory failed"))
}

fn random_name() -> Result<String, Error> {
    let mut buf = [0; 8];
    let mut rng = File::open("/dev/urandom")?;
    rng.read_exact(&mut buf)?;

    Ok(format!("cuddld-{}", hex_encode(&buf)))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut buffer = String::new();
    for byte in bytes {
        buffer.push_str(&format!("{byte:0>2x}"));
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode() {
        assert_eq!("00", hex_encode(&[0]));
        assert_eq!("0001ff", hex_encode(&[0, 1, 255]));
    }

    #[test]
    fn test_random_name() {
        assert_ne!(random_name().unwrap(), random_name().unwrap());
    }

    #[test]
    fn test_create_temp_dir() {
        let dir1 = create_temp_dir().unwrap();
        let dir2 = create_temp_dir().unwrap();

        assert_ne!(dir1, dir2);

        std::fs::remove_dir(dir1).unwrap();
        std::fs::remove_dir(dir2).unwrap();
    }
}
