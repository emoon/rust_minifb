#pragma once

typedef struct SharedData {
    unsigned int bg_color;
    unsigned int scale_mode;
    unsigned int width;
    unsigned int height;
    float mouse_x;
    float mouse_y;
    float scroll_x;
    float scroll_y;
    unsigned char mouse_state[8];
} SharedData;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct DrawParameters {
    void* buffer;
    unsigned int bg_color;
    int buffer_width;
    int buffer_height;
    int buffer_stride;
    int scale_mode;
} DrawParameters;
