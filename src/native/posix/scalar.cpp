#include <stdint.h>
#include <stdio.h>

extern "C" void Image_resize_linear_c(uint32_t* target, const uint32_t* source, int w, int h, int s, int w2, int h2) {
    if (w2 <= 0) { w2 = 1; }
    if (h2 <= 0) { h2 = 1; }

    float x_ratio = ((float)(w)) / w2;
    float y_ratio = ((float)(h)) / h2;
    int step_x = x_ratio * 1024.0f;
    int step_y = y_ratio * 1024.0f;
    int fixed_y = 0;

    for (int i = 0; i < h2; i++) {
        const int y = (fixed_y >> 10) * s;
        int fixed_x = 0;
        for (int j = 0; j < w2; j++) {
            int x = fixed_x >> 10;
            int index = (y + x);
            *target++ = source[index];
            fixed_x += step_x;
        }

        fixed_y += step_y;
    }
}

static void resize_linear_c_stride(uint32_t* target, const uint32_t* source, int w, int h, int s, int w2, int h2, int stride) {
    if (w2 <= 0) { w2 = 1; }
    if (h2 <= 0) { h2 = 1; }

    float x_ratio = ((float)(w)) / w2;
    float y_ratio = ((float)(h)) / h2;
    int step_x = x_ratio * 1024.0f;
    int step_y = y_ratio * 1024.0f;
    int fixed_y = 0;
    int stride_step = stride - w2;

    for (int i = 0; i < h2; i++) {
        const int y = (fixed_y >> 10) * s;
        int fixed_x = 0;
        for (int j = 0; j < w2; j++) {
            int x = fixed_x >> 10;
            int index = (y + x);
            *target++ = source[index];
            fixed_x += step_x;
        }

        target += stride_step;
        fixed_y += step_y;
    }
}

extern "C" void Image_resize_linear_aspect_fill_c(
    uint32_t* target,
    const uint32_t* source,
    int w, int h, int s,
    int window_width, int window_height, uint32_t bg_clear)
{
    // TODO: Optimize by only clearing the areas the image blit doesn't fill
    for (int i = 0; i < window_width * window_height; ++i) {
        target[i] = bg_clear;
    }

    float buffer_aspect = float(w) / float(h);
    float win_aspect = float(window_width) / float(window_height);

    if (buffer_aspect > win_aspect) {
        int new_height = (int)(window_width / buffer_aspect);
        int offset = (new_height - window_height) / -2;
        Image_resize_linear_c(
            target + (offset * window_width),
            source, w, h, s,
            window_width, new_height);
    } else {
        int new_width = (int)(window_height * buffer_aspect);
        int offset = (new_width - window_width) / -2;
        resize_linear_c_stride(
            target + offset, source, w, h, s,
            new_width, window_height, window_width);
    }
}

extern "C" void Image_center(
    uint32_t* target,
    const uint32_t* source,
    int w, int h, int s,
    int window_width, int window_height, uint32_t bg_clear)
{
    // TODO: Optimize by only clearing the areas the image blit doesn't fill
    for (int i = 0; i < window_width * window_height; ++i) {
        target[i] = bg_clear;
    }

    if (h > window_height) {
        int y_offset = (h - window_height) / 2;
        int new_height = h - y_offset;
        source += y_offset * s;

        if (new_height > window_height)
            new_height = window_height;

        if (w > window_width) {
            int x_offset = (w - window_width) / 2;
            source += x_offset;

            for (int y = 0; y < window_height; ++y) {
                for (int x = 0; x < window_width; ++x) {
                    *target++ = *source++;
                }
                source += (s - window_width);
            }
        } else {
            int x_offset = (window_width - w) / 2;

            for (int y = 0; y < new_height; ++y) {
                target += x_offset;

                for (int x = 0; x < w; ++x) {
                    *target++ = *source++;
                }

                target += (window_width - (w + x_offset));
                source += s - w;
            }
        }

    } else {
        int y_offset = (window_height - h) / 2;
        target += y_offset * window_width;

        if (w > window_width) {
            int x_offset = (w - window_width) / 2;
            source += x_offset;

            for (int y = 0; y < h; ++y) {
                for (int x = 0; x < window_width; ++x) {
                    *target++ = *source++;
                }
                source += (s - window_width);
            }
        } else {
            int x_offset = (window_width - w) / 2;
            target += x_offset;

            for (int y = 0; y < h; ++y) {
                for (int x = 0; x < w; ++x) {
                    *target++ = *source++;
                }

                target += (window_width - w);
                source += s - w;
            }
        }
    }
}

extern "C" void Image_upper_left(
    uint32_t* target,
    const uint32_t* source,
    int w, int h, int s,
    int window_width, int window_height, uint32_t bg_clear)
{
    // TODO: Optimize by only clearing the areas the image blit doesn't fill
    for (int i = 0; i < window_width * window_height; ++i) {
        target[i] = bg_clear;
    }

    if (h > window_height) {
        int y_offset = (h - window_height) / 2;
        int new_height = h - y_offset;

        if (w > window_width) {
            for (int y = 0; y < window_height; ++y) {
                for (int x = 0; x < window_width; ++x) {
                    *target++ = *source++;
                }
                source += (s - window_width);
            }
        } else {
            for (int y = 0; y < new_height; ++y) {
                for (int x = 0; x < w; ++x) {
                    *target++ = *source++;
                }

                target += (window_width - w);
                source += s - w;
            }
        }

    } else {
        if (w > window_width) {
            for (int y = 0; y < h; ++y) {
                for (int x = 0; x < window_width; ++x) {
                    *target++ = *source++;
                }
                source += (s - window_width);
            }
        } else {
            for (int y = 0; y < h; ++y) {
                for (int x = 0; x < w; ++x) {
                    *target++ = *source++;
                }

                target += (window_width - w);
                source += s - w;
            }
        }
    }
}

