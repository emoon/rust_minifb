#include <stdint.h>
#include <stddef.h>

// Nearest-neighbour source index for destination pixel j is exactly
// floor(j * src / dst). Walking an integer remainder gets there without a
// per-pixel divide, and without the drift a fixed-point step accumulates: a
// 10.10 step quantises the ratio to 1/1024, which on a non-integer scale
// factor lands on the wrong source pixel for over half a row and drifts by
// several pixels by the right edge. The GPU path in `os/posix/gl.rs` samples
// the same ratio at fragment centres, so this is also what keeps the two paths
// within half a destination pixel of each other instead of drifting apart.
//
// Splitting the ratio into `src / dst` and `src % dst` keeps that to one add
// and at most one carry per destination pixel, so a heavy downscale costs the
// same as any other: accumulating `src` and subtracting `dst` until it fits
// would instead carry `src / dst` times per pixel, which is 220M iterations
// for the 2.2M -> 100 case the tests cover.
//
// Remainders are 64-bit because both dimensions are `uint32_t`: `rem + err`
// would wrap a 32-bit accumulator, and `check_buffer_size` happily accepts a
// 2.1M x 1 buffer from safe Rust.

static void image_clear(uint32_t* dst, const uint32_t dst_width, const uint32_t dst_height, const uint32_t bg_clear) {
    const size_t count = (size_t)dst_width * (size_t)dst_height;
    for (size_t i = 0; i < count; ++i) {
        dst[i] = bg_clear;
    }
}

void image_resize_linear(
    uint32_t* dst,
    const uint32_t dst_width,
    const uint32_t dst_height,
    const uint32_t* src,
    const uint32_t src_width,
    const uint32_t src_height,
    const uint32_t src_stride
) {
    if (dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0) {
        return;
    }

    const uint32_t max_x = src_width - 1;
    const uint32_t max_y = src_height - 1;
    const uint32_t step_x = src_width / dst_width;
    const uint32_t step_y = src_height / dst_height;
    const uint32_t err_x = src_width % dst_width;
    const uint32_t err_y = src_height % dst_height;
    uint32_t sy = 0;
    uint64_t rem_y = 0;

    for (uint32_t i = 0; i < dst_height; i++) {
        // `sy` cannot pass `max_y` for i < dst_height; the clamp only bounds
        // the read if a caller ever gets those two out of step.
        const size_t y = (size_t)(sy > max_y ? max_y : sy) * (size_t)src_stride;
        uint32_t sx = 0;
        uint64_t rem_x = 0;

        for (uint32_t j = 0; j < dst_width; j++) {
            *dst++ = src[y + (sx > max_x ? max_x : sx)];
            sx += step_x;
            rem_x += err_x;
            if (rem_x >= dst_width) {
                rem_x -= dst_width;
                sx++;
            }
        }

        sy += step_y;
        rem_y += err_y;
        if (rem_y >= dst_height) {
            rem_y -= dst_height;
            sy++;
        }
    }
}

static void image_resize_linear_stride(
    uint32_t* dst,
    const uint32_t dst_width,
    const uint32_t dst_height,
    const uint32_t* src,
    const uint32_t src_width,
    const uint32_t src_height,
    const uint32_t src_stride,
    const uint32_t stride
) {
    if (dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0) {
        return;
    }

    const uint32_t max_x = src_width - 1;
    const uint32_t max_y = src_height - 1;
    const uint32_t step_x = src_width / dst_width;
    const uint32_t step_y = src_height / dst_height;
    const uint32_t err_x = src_width % dst_width;
    const uint32_t err_y = src_height % dst_height;
    const ptrdiff_t stride_step = (ptrdiff_t)stride - (ptrdiff_t)dst_width;
    uint32_t sy = 0;
    uint64_t rem_y = 0;

    for (uint32_t i = 0; i < dst_height; i++) {
        // `sy` cannot pass `max_y` for i < dst_height; the clamp only bounds
        // the read if a caller ever gets those two out of step.
        const size_t y = (size_t)(sy > max_y ? max_y : sy) * (size_t)src_stride;
        uint32_t sx = 0;
        uint64_t rem_x = 0;

        for (uint32_t j = 0; j < dst_width; j++) {
            *dst++ = src[y + (sx > max_x ? max_x : sx)];
            sx += step_x;
            rem_x += err_x;
            if (rem_x >= dst_width) {
                rem_x -= dst_width;
                sx++;
            }
        }

        dst += stride_step;
        sy += step_y;
        rem_y += err_y;
        if (rem_y >= dst_height) {
            rem_y -= dst_height;
            sy++;
        }
    }
}

void image_resize_linear_aspect_fill(
    uint32_t* dst,
    const uint32_t dst_width,
    const uint32_t dst_height,
    const uint32_t* src,
    const uint32_t src_width,
    const uint32_t src_height,
    const uint32_t src_stride,
    const uint32_t bg_clear
) {
    // TODO: Optimize by only clearing the areas the image blit doesn't fill
    image_clear(dst, dst_width, dst_height, bg_clear);

    if (dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0) {
        return;
    }

    const float buffer_aspect = (float)(src_width) / (float)(src_height);
    const float win_aspect = (float)(dst_width) / (float)(dst_height);

    if (buffer_aspect > win_aspect) {
        // Letterboxed: full width, centered vertically.
        uint32_t new_height = (uint32_t)((float)dst_width / buffer_aspect);
        if (new_height > dst_height) new_height = dst_height;
        if (new_height == 0) new_height = 1;
        const uint32_t y_offset = (dst_height - new_height) / 2;

        image_resize_linear(
            dst + (size_t)y_offset * (size_t)dst_width,
            dst_width, new_height,
            src, src_width, src_height, src_stride
        );
    } else {
        // Pillarboxed: full height, centered horizontally.
        uint32_t new_width = (uint32_t)((float)dst_height * buffer_aspect);
        if (new_width > dst_width) new_width = dst_width;
        if (new_width == 0) new_width = 1;
        const uint32_t x_offset = (dst_width - new_width) / 2;

        image_resize_linear_stride(
            dst + x_offset,
            new_width, dst_height,
            src, src_width, src_height, src_stride,
            dst_width
        );
    }
}

void image_center(
    uint32_t* dst,
    const uint32_t dst_width,
    const uint32_t dst_height,
    const uint32_t* src,
    const uint32_t src_width,
    const uint32_t src_height,
    const uint32_t src_stride,
    const uint32_t bg_clear
) {
    // TODO: Optimize by only clearing the areas the image blit doesn't fill
    image_clear(dst, dst_width, dst_height, bg_clear);

    if (dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0) {
        return;
    }

    if (src_height > dst_height) {
        const uint32_t y_offset = (src_height - dst_height) / 2;
        uint32_t new_height = src_height - y_offset;
        src += (size_t)y_offset * (size_t)src_stride;

        if (new_height > dst_height)
            new_height = dst_height;

        if (src_width > dst_width) {
            const uint32_t x_offset = (src_width - dst_width) / 2;
            src += x_offset;

            for (uint32_t y = 0; y < new_height; ++y) {
                for (uint32_t x = 0; x < dst_width; ++x) {
                    *dst++ = *src++;
                }
                src += (src_stride - dst_width);
            }
        } else {
            const uint32_t x_offset = (dst_width - src_width) / 2;

            for (uint32_t y = 0; y < new_height; ++y) {
                dst += x_offset;

                for (uint32_t x = 0; x < src_width; ++x) {
                    *dst++ = *src++;
                }
                dst += (dst_width - (src_width + x_offset));
                src += src_stride - src_width;
            }
        }
    } else {
        const uint32_t y_offset = (dst_height - src_height) / 2;
        dst += (size_t)y_offset * (size_t)dst_width;

        if (src_width > dst_width) {
            const uint32_t x_offset = (src_width - dst_width) / 2;
            src += x_offset;

            for (uint32_t y = 0; y < src_height; ++y) {
                for (uint32_t x = 0; x < dst_width; ++x) {
                    *dst++ = *src++;
                }
                src += (src_stride - dst_width);
            }
        } else {
            const uint32_t x_offset = (dst_width - src_width) / 2;
            dst += x_offset;

            for (uint32_t y = 0; y < src_height; ++y) {
                for (uint32_t x = 0; x < src_width; ++x) {
                    *dst++ = *src++;
                }
                dst += (dst_width - src_width);
                src += src_stride - src_width;
            }
        }
    }
}

void image_upper_left(
    uint32_t* dst,
    const uint32_t dst_width,
    const uint32_t dst_height,
    const uint32_t* src,
    const uint32_t src_width,
    const uint32_t src_height,
    const uint32_t src_stride,
    const uint32_t bg_clear
) {
    // TODO: Optimize by only clearing the areas the image blit doesn't fill
    image_clear(dst, dst_width, dst_height, bg_clear);

    if (dst_width == 0 || dst_height == 0 || src_width == 0 || src_height == 0) {
        return;
    }

    // Anchored top-left, so the visible region is simply the leading
    // min(src, dst) rows and columns of the source.
    const uint32_t copy_height = src_height < dst_height ? src_height : dst_height;
    const uint32_t copy_width = src_width < dst_width ? src_width : dst_width;

    for (uint32_t y = 0; y < copy_height; ++y) {
        for (uint32_t x = 0; x < copy_width; ++x) {
            *dst++ = *src++;
        }
        dst += (dst_width - copy_width);
        src += (src_stride - copy_width);
    }
}
