#pragma once

typedef struct SharedData {
    unsigned int width;
    unsigned int height;
    float mouse_x;
    float mouse_y;
    float scroll_x;
    float scroll_y;
    unsigned char mouse_state[8];
} SharedData;

