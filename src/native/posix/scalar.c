#include <stdint.h>
#include <stddef.h>

// Fixed-point coordinates are accumulated in 64-bit: a source dimension above
// 2^21 overflows a 32-bit 10.10 accumulator and wraps to a negative index,
// which is reachable from the safe Rust API (`check_buffer_size` happily
// accepts a 2.1M x 1 buffer).
#define FP_SHIFT 10
#define FP_ONE (1 << FP_SHIFT)

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

    const float x_ratio = (float)(src_width) / (float)(dst_width);
    const float y_ratio = (float)(src_height) / (float)(dst_height);
    const int64_t step_x = (int64_t)(x_ratio * (float)FP_ONE);
    const int64_t step_y = (int64_t)(y_ratio * (float)FP_ONE);
    const int64_t max_x = (int64_t)src_width - 1;
    const int64_t max_y = (int64_t)src_height - 1;
    int64_t fixed_y = 0;

    for (uint32_t i = 0; i < dst_height; i++) {
        int64_t sy = fixed_y >> FP_SHIFT;
        if (sy > max_y) sy = max_y;
        const int64_t y = sy * (int64_t)src_stride;
        int64_t fixed_x = 0;
        for (uint32_t j = 0; j < dst_width; j++) {
            int64_t x = fixed_x >> FP_SHIFT;
            if (x > max_x) x = max_x;
            *dst++ = src[y + x];
            fixed_x += step_x;
        }
        fixed_y += step_y;
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

    const float x_ratio = (float)(src_width) / (float)(dst_width);
    const float y_ratio = (float)(src_height) / (float)(dst_height);
    const int64_t step_x = (int64_t)(x_ratio * (float)FP_ONE);
    const int64_t step_y = (int64_t)(y_ratio * (float)FP_ONE);
    const int64_t max_x = (int64_t)src_width - 1;
    const int64_t max_y = (int64_t)src_height - 1;
    const ptrdiff_t stride_step = (ptrdiff_t)stride - (ptrdiff_t)dst_width;
    int64_t fixed_y = 0;

    for (uint32_t i = 0; i < dst_height; i++) {
        int64_t sy = fixed_y >> FP_SHIFT;
        if (sy > max_y) sy = max_y;
        const int64_t y = sy * (int64_t)src_stride;
        int64_t fixed_x = 0;
        for (uint32_t j = 0; j < dst_width; j++) {
            int64_t x = fixed_x >> FP_SHIFT;
            if (x > max_x) x = max_x;
            *dst++ = src[y + x];
            fixed_x += step_x;
        }
        dst += stride_step;
        fixed_y += step_y;
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
