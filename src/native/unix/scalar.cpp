#include <stdint.h>
#include <stdio.h>

/*
// TODO: Write SSE version as well
http://stereopsis.com/doubleblend.html
64-bit loads (loadl_epi64) at source[index] (gives you two horizontal pixels) and source[index + w]
2. punpcklqdq (unpack_lo_epi64) two smush them into one 128-bit vector
3. pshufb (shuffle_epi8) to reorder the bytes so you have (r00,r01,r10,r11, g00,g01,g10,g11, ...)
4. pmaddubsw (maddubs_epi16 I think?) with (64-xweight,xweight, 64-xweight,xweight, ...) to do the horizontal lerps
5. pmaddwd (madd_epi16) with 16-bit (64-yweight,yweight, 64-yweight,yweight, ...) to do the vertical lerps
6. paddd (add_epi32) with broadcast 32-bit (1 << 11), then psrld (srli_epi32) with 12 to get the rounded results (still in 32 bits/lane)
and after that, packssdw (packs_epi32) and then packuswb (packus_epi16) to get back to 4 bytes of r, g, b, a
it's not ideal to go up all the way to 32 bits intermediate but it is precise and probably the easiest way
*/

// TODO: Convert to integer
extern "C" void Image_resize_bilinear_c(uint32_t* target, const uint32_t* source, int w, int h, int s, int w2, int h2) {
    int a, b, c, d, x, y, index;
    if (w2 <= 0) { w2 = 1; }
    if (h2 <= 0) { h2 = 1; }

    float x_ratio = ((float)(w - 1)) / w2;
    float y_ratio = ((float)(h - 1)) / h2;
    float x_diff, y_diff, blue, red, green;

    for (int i = 0; i < h2; i++) {
        for (int j = 0; j < w2; j++) {
            x = (int)(x_ratio * j);
            y = (int)(y_ratio * i);
            x_diff = (x_ratio * j) - x;
            y_diff = (y_ratio * i) - y;
            index = (y * s + x);
            a = source[index];
            b = source[index + 1];
            c = source[index + w];
            d = source[index + w + 1];

            // blue element
            // Yb = Ab(1-w)(1-h) + Bb(w)(1-h) + Cb(h)(1-w) + Db(wh)
            blue = (a&0xff)*(1-x_diff)*(1-y_diff) + (b&0xff)*(x_diff)*(1-y_diff) +
                   (c&0xff)*(y_diff)*(1-x_diff)   + (d&0xff)*(x_diff*y_diff);

            // green element
            // Yg = Ag(1-w)(1-h) + Bg(w)(1-h) + Cg(h)(1-w) + Dg(wh)
            green = ((a>>8)&0xff)*(1-x_diff)*(1-y_diff) + ((b>>8)&0xff)*(x_diff)*(1-y_diff) +
                    ((c>>8)&0xff)*(y_diff)*(1-x_diff)   + ((d>>8)&0xff)*(x_diff*y_diff);

            // red element
            // Yr = Ar(1-w)(1-h) + Br(w)(1-h) + Cr(h)(1-w) + Dr(wh)
            red = ((a>>16)&0xff)*(1-x_diff)*(1-y_diff) + ((b>>16)&0xff)*(x_diff)*(1-y_diff) +
                  ((c>>16)&0xff)*(y_diff)*(1-x_diff)   + ((d>>16)&0xff)*(x_diff*y_diff);

            *target++ = ((((int)red) << 16) & 0xff0000) | ((((int)green) << 8) & 0xff00) | ((int)blue);
        }
    }
}

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

extern "C" void Image_resize_linear_c_stride(uint32_t* target, const uint32_t* source, int w, int h, int s, int w2, int h2, int stride) {
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
        Image_resize_linear_c_stride(
            target + offset, source, w, h, s,
            new_width, window_height, window_width);
    }
}


