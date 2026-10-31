//! Embed vendored Phosphor's exact MIT notice only when an icon was rendered.
pub(crate) const PHOSPHOR_LICENSE: &str = include_str!("../assets/phosphor/LICENSE");
pub(crate) const MARKER: &str = "<metadata id=\"phosphor-license\">";

pub(crate) fn uses_phosphor(svg: &str) -> bool {
    svg.contains(MARKER)
}

pub(crate) fn preserve_svg_notice(source: &str, outlined: String) -> Result<String, String> {
    if !uses_phosphor(source) || uses_phosphor(&outlined) {
        return Ok(outlined);
    }
    let start = source.find(MARKER).ok_or("SVG Phosphor metadata missing")?;
    let end = source[start..]
        .find("</metadata>")
        .ok_or("SVG Phosphor metadata unclosed")?
        + start
        + "</metadata>".len();
    let root = outlined.find("<svg ").ok_or("outlined SVG root missing")?;
    let head = outlined[root..]
        .find('>')
        .ok_or("outlined SVG root unclosed")?
        + root
        + 1;
    let mut result = outlined;
    result.insert_str(head, &source[start..end]);
    Ok(result)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}

pub(crate) fn png_notice(png: Vec<u8>) -> Result<Vec<u8>, String> {
    if png.len() < 33 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
        return Err("cannot embed Phosphor license in invalid PNG".into());
    }
    let mut data = b"License\0\0\0\0\0".to_vec(); // iTXt: uncompressed, no language/translated keyword
    data.extend_from_slice(PHOSPHOR_LICENSE.as_bytes());
    let mut part = Vec::with_capacity(data.len() + 12);
    part.extend_from_slice(&(data.len() as u32).to_be_bytes());
    part.extend_from_slice(b"iTXt");
    part.extend_from_slice(&data);
    part.extend_from_slice(&crc32(&part[4..]).to_be_bytes());
    let mut result = Vec::with_capacity(png.len() + part.len());
    result.extend_from_slice(&png[..33]);
    result.extend_from_slice(&part);
    result.extend_from_slice(&png[33..]);
    Ok(result)
}

pub(crate) fn pdf_notice(pdf: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut document = lopdf::Document::load_mem(&pdf)
        .map_err(|e| format!("cannot read PDF for Phosphor attribution: {e}"))?;
    let info = match document
        .trailer
        .get(b"Info")
        .and_then(lopdf::Object::as_reference)
    {
        Ok(id) => id,
        Err(_) => {
            let id = document.new_object_id();
            document
                .objects
                .insert(id, lopdf::Object::Dictionary(lopdf::Dictionary::new()));
            document.trailer.set("Info", id);
            id
        }
    };
    document
        .get_object_mut(info)
        .and_then(lopdf::Object::as_dict_mut)
        .map_err(|e| format!("cannot write PDF attribution dictionary: {e}"))?
        .set("Subject", lopdf::Object::string_literal(PHOSPHOR_LICENSE));
    let mut result = Vec::new();
    document
        .save_to(&mut result)
        .map_err(|e| format!("cannot embed Phosphor attribution in PDF: {e}"))?;
    Ok(result)
}
