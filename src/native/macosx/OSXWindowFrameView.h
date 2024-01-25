#import <Cocoa/Cocoa.h>
#import <MetalKit/MetalKit.h>
#include "shared_data.h"

// Number of textures in flight (tripple buffered)
const int MaxBuffersInFlight = 3;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct DrawState {
    int texture_width;
    int texture_height;
    id<MTLBuffer> vertex_buffer;
    id<MTLTexture> texture;
} DrawState;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct Vertex {
    float x,y;
    float u,v;
} Vertex;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct DelayedTextureDelete {
    id<MTLTexture> texture;
    int frame_count;
} DelayedTextureDelete;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

@interface WindowViewController : NSViewController<MTKViewDelegate> {
    @public DrawState m_draw_state[MaxBuffersInFlight];
    @public DelayedTextureDelete m_delayed_delete_textures[MaxBuffersInFlight];
    @public int m_current_buffer;
    @public DrawParameters* m_draw_parameters;
    @public float m_width;
    @public float m_height;
    // Used for syncing with CPU/GPU
    @public dispatch_semaphore_t m_semaphore;
}
@end

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

@interface OSXWindowFrameView : NSView {
    @public WindowViewController* m_view_controller;
    //@public int scale;
    //@public int width;
    //@public int height;
    //@public void* draw_buffer;
    @private NSTrackingArea* trackingArea;
}
@end
