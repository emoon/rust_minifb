const WINDOW_BORDERLESS: u32 = 1 << 1; 
const WINDOW_RESIZE: u32 = 1 << 2; 
const WINDOW_TITLE: u32 = 1 << 3; 

use WindowOptions;

//
// Construct a bitmask of flags (sent to backends) from WindowOpts
//
pub fn get_flags(opts: WindowOptions) -> u32 {
    let mut flags = 0u32;

    if opts.borderless {
        flags |= WINDOW_BORDERLESS;
    }

    if opts.title {
        flags |= WINDOW_TITLE;
    }

    if opts.resize {
        flags |= WINDOW_RESIZE;
    }

    flags
}
