use error::Error;
use Result;

pub fn check_buffer_size(
    window_width: usize,
    window_height: usize,
    scale: usize,
    buffer: &[u32],
) -> Result<()> {
    let buffer_size = buffer.len() * 4; // len is the number of entries so * 4 as we want bytes
    let required_buffer_size = (window_width / scale) * (window_height / scale) * 4; // * 4 for 32-bit buffer

    if buffer_size < required_buffer_size {
        let err = format!("Update failed because input buffer is too small. Required size for {} x {} window ({}x scale) is {} bytes but the size of the input buffer has the size {} bytes",
                           window_width, window_height, scale, required_buffer_size, buffer_size);
        Err(Error::UpdateFailed(err))
    } else {
        Ok(())
    }
}
