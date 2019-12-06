#include <stdint.h>
#include <stdio.h>

// TODO: Convert to integer
// TODO: Write SSE version as well
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

