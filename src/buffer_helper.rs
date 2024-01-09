use crate::error::Error;
use crate::Result;

pub fn check_buffer_size(
    buffer: &[u32],
    buffer_width: usize,
    buffer_height: usize,
    buffer_stride: usize,
) -> Result<()> {
    let width = usize::max(buffer_width, buffer_stride);
    let buffer_size = buffer.len() * std::mem::size_of::<u32>();
    let required_buffer_size = width * buffer_height * std::mem::size_of::<u32>(); // * 4 (size of u32) for 32-bit buffer

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
