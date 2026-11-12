use crate::exifutils::LittleEndian;
use crate::{exifutils, BracketEXIFInformation, Result};
use exif::Context;

/// Value of the `SequenceNumber` tag (0xb04a) if the camera does not provide a sequence number.
const SEQUENCE_NUMBER_NOT_AVAILABLE: u32 = 0xffff;

pub fn get_bracketing_info(maker_note_data: &[u8]) -> Result<Option<BracketEXIFInformation>> {
    if maker_note_data.starts_with("SONY DSC \0\0\0".as_bytes())
        || maker_note_data.starts_with("SONY CAM \0\0\0".as_bytes())
    {
        let maker_note =
            exifutils::parse_ifd::<LittleEndian>(maker_note_data, 12, Context::Exif, 0)?;

        let sequence_number = match maker_note.get(&0xb04a) {
            None => return Ok(None),
            Some(field) => match field.value.as_uint()?.get(0) {
                Some(v) => v,
                None => return Ok(None),
            },
        };

        if sequence_number > 0 && sequence_number != SEQUENCE_NUMBER_NOT_AVAILABLE {
            return Ok(Some(BracketEXIFInformation {
                index: sequence_number,
            }));
        }
    }

    Ok(None)
}
