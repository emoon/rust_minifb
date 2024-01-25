#include <stdint.h>
#include <stdio.h>

extern "C" void image_resize_linear(
    uint32_t* dst,
    const uint32_t dst_width,
    const uint32_t dst_height,
    const uint32_t* src,
    const uint32_t src_width,
    const uint32_t src_height,
    const uint32_t src_stride
) {
    const float x_ratio = float(src_width) / float(dst_width);
    const float y_ratio = float(src_height) / float(dst_height);
    const int step_x = x_ratio * 1024.0f;
    const int step_y = y_ratio * 1024.0f;
    int fixed_y = 0;

    for (uint32_t i = 0; i < dst_height; i++) {
        const int y = (fixed_y >> 10) * src_stride;
        int fixed_x = 0;
        for (uint32_t j = 0; j < dst_width; j++) {
            int x = fixed_x >> 10;
            int index = (y + x);
            *dst++ = src[index];
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
    const float x_ratio = float(src_width) / float(dst_width);
    const float y_ratio = float(src_height) / float(dst_height);
    const int step_x = x_ratio * 1024.0f;
    const int step_y = y_ratio * 1024.0f;
    const int stride_step = stride - dst_width;
    int fixed_y = 0;

    for (uint32_t i = 0; i < dst_height; i++) {
        const int y = (fixed_y >> 10) * src_stride;
        int fixed_x = 0;
        for (uint32_t j = 0; j < dst_width; j++) {
            const int x = fixed_x >> 10;
            const int index = (y + x);
            *dst++ = src[index];
            fixed_x += step_x;
        }
        dst += stride_step;
        fixed_y += step_y;
    }
}

extern "C" void image_resize_linear_aspect_fill(
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
    for (uint32_t i = 0; i < dst_width * dst_height; ++i) {
        dst[i] = bg_clear;
    }

    const float buffer_aspect = float(src_width) / float(src_height);
    const float win_aspect = float(dst_width) / float(dst_height);

    if (buffer_aspect > win_aspect) {
        const uint32_t new_height = uint32_t(dst_width / buffer_aspect);
        const int offset = (new_height - dst_height) / -2;
        image_resize_linear(
            dst + (offset * dst_width),
            dst_width, new_height,
            src, src_width, src_height, src_stride
        );
    } else {
        const uint32_t new_width = uint32_t(dst_height * buffer_aspect);
        const int offset = (new_width - dst_width) / -2;
        image_resize_linear_stride(
            dst + offset,
            dst_height, dst_width,
            src, src_width, src_height, src_stride,
            new_width
        );
    }
}

extern "C" void image_center(
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
    for (uint32_t i = 0; i < dst_width * dst_height; ++i) {
        dst[i] = bg_clear;
    }

    if (src_height > dst_height) {
        const int y_offset = (src_height - dst_height) / 2;
        uint32_t new_height = src_height - y_offset;
        src += y_offset * src_stride;

        if (new_height > dst_height)
            new_height = dst_height;

        if (src_width > dst_width) {
            const int x_offset = (src_width - dst_width) / 2;
            src += x_offset;

            for (uint32_t y = 0; y < dst_height; ++y) {
                for (uint32_t x = 0; x < dst_width; ++x) {
                    *dst++ = *src++;
                }
                src += (src_stride - dst_width);
            }
        } else {
            const int x_offset = (dst_width - src_width) / 2;

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
        const int y_offset = (dst_height - src_height) / 2;
        dst += y_offset * dst_width;

        if (src_width > dst_width) {
            const int x_offset = (src_width - dst_width) / 2;
            src += x_offset;

            for (uint32_t y = 0; y < src_height; ++y) {
                for (uint32_t x = 0; x < dst_width; ++x) {
                    *dst++ = *src++;
                }
                src += (src_stride - dst_width);
            }
        } else {
            const int x_offset = (dst_width - src_width) / 2;
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

extern "C" void image_upper_left(
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
    for (uint32_t i = 0; i < dst_width * dst_height; ++i) {
        dst[i] = bg_clear;
    }

    if (src_height > dst_height) {
        const int y_offset = (src_height - dst_height) / 2;
        const uint32_t new_height = src_height - y_offset;

        if (src_width > dst_width) {
            for (uint32_t y = 0; y < dst_height; ++y) {
                for (uint32_t x = 0; x < dst_width; ++x) {
                    *dst++ = *src++;
                }
                src += (src_stride - dst_width);
            }
        } else {
            for (uint32_t y = 0; y < new_height; ++y) {
                for (uint32_t x = 0; x < src_width; ++x) {
                    *dst++ = *src++;
                }
                dst += (dst_width - src_width);
                src += src_stride - src_width;
            }
        }
    } else {
        if (src_width > dst_width) {
            for (uint32_t y = 0; y < src_height; ++y) {
                for (uint32_t x = 0; x < dst_width; ++x) {
                    *dst++ = *src++;
                }
                src += (src_stride - dst_width);
            }
        } else {
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
