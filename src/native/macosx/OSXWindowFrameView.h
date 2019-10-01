#import <Cocoa/Cocoa.h>
#import <MetalKit/MetalKit.h>

// Number of textures in flight (tripple buffered)
const int MaxBuffersInFlight = 3;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

@interface WindowViewController : NSViewController<MTKViewDelegate>
{
	@public id<MTLTexture> m_texture_buffers[MaxBuffersInFlight];
	@public id<MTLTexture> m_delayed_delete_textures[MaxBuffersInFlight];
	@public int m_current_buffer;
	@public void* m_draw_buffer;
	@public int m_width;
	@public int m_height;
	@public int m_delayed_delete_count;
	// Used for syncing with CPU/GPU
	@public dispatch_semaphore_t m_semaphore;
}

@end

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

@interface OSXWindowFrameView : NSView
{
	@public WindowViewController* m_view_controller;
	//@public int scale;
	//@public int width;
	//@public int height;
	//@public void* draw_buffer;
	@private NSTrackingArea* trackingArea;
}

@end

