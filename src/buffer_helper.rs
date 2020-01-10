use crate::error::Error;
use crate::Result;

pub fn check_buffer_size(
    buffer_width: usize,
    buffer_height: usize,
    buffer_stride: usize,
    buffer: &[u32],
) -> Result<()> {
    let width = usize::max(buffer_width, buffer_stride);
    let buffer_size = buffer.len() * 4; // len is the number of entries so * 4 as we want bytes
    let required_buffer_size = width * buffer_height * 4; // * 4 for 32-bit buffer

    if buffer_size < required_buffer_size {
        let err = format!(
            "Update failed because input buffer is too small. Required size for {} ({} stride) x {} buffer is {}
            bytes but the size of the input buffer has the size {} bytes",
            buffer_width, buffer_stride, buffer_height, required_buffer_size, buffer_size);
        Err(Error::UpdateFailed(err))
    } else {
        Ok(())
    }
}
