//! Font files embedded into exported SVGs so they look right anywhere.

pub fn bytes(file: &str) -> Option<&'static [u8]> {
    Some(match file {
        "open-sans-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/open-sans-latin-400-normal.woff2")
        }
        "open-sans-latin-700-normal.woff2" => {
            include_bytes!("../../../ui/fonts/open-sans-latin-700-normal.woff2")
        }
        "montserrat-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/montserrat-latin-400-normal.woff2")
        }
        "montserrat-latin-700-normal.woff2" => {
            include_bytes!("../../../ui/fonts/montserrat-latin-700-normal.woff2")
        }
        "lora-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/lora-latin-400-normal.woff2")
        }
        "lora-latin-700-normal.woff2" => {
            include_bytes!("../../../ui/fonts/lora-latin-700-normal.woff2")
        }
        "caveat-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/caveat-latin-400-normal.woff2")
        }
        "caveat-latin-700-normal.woff2" => {
            include_bytes!("../../../ui/fonts/caveat-latin-700-normal.woff2")
        }
        "patrick-hand-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/patrick-hand-latin-400-normal.woff2")
        }
        "pacifico-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/pacifico-latin-400-normal.woff2")
        }
        "source-code-pro-latin-400-normal.woff2" => {
            include_bytes!("../../../ui/fonts/source-code-pro-latin-400-normal.woff2")
        }
        "source-code-pro-latin-700-normal.woff2" => {
            include_bytes!("../../../ui/fonts/source-code-pro-latin-700-normal.woff2")
        }
        _ => return None,
    })
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"hello world"), "aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn every_catalogue_file_is_embedded_and_is_woff2() {
        for f in mindmap_core::fonts::FONTS {
            for (_, file) in f.files {
                let b = bytes(file).unwrap_or_else(|| panic!("{file} missing from fontdata"));
                assert_eq!(&b[..4], b"wOF2", "{file} is not woff2");
            }
        }
    }
}
