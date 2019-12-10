#import "OSXWindowFrameView.h"
#import "OSXWindow.h"
#import <MetalKit/MetalKit.h>

id<MTLDevice> g_metal_device;
id<MTLCommandQueue> g_command_queue;
id<MTLLibrary> g_library;
id<MTLRenderPipelineState> g_pipeline_state;

enum ScaleMode {
    ScaleMode_Stretch,
    ScaleMode_AspectRatioStretch,
    ScaleMode_Center,
    ScaleMode_UpperLeft,
};

typedef struct Box {
	int x, y, width, height;
} Box;

static void gen_normalized(Vertex* output, const Box* box, float x, float y, float u, float v) {
	// data gets normalized in the shader
	float pos_x = box->x * x;
	float pos_y = box->y * y;
	float width = box->width * x;
	float height = box->height * y;

	output[0].x = pos_x;
	output[0].y = pos_y;
	output[1].x = width;
	output[1].y = pos_y;
	output[2].x = width;
	output[2].y = height;

	output[3].x = pos_x;
	output[3].y = pos_y;
	output[4].x = width;
	output[4].y = height;
	output[5].x = pos_x;
	output[5].y = height;
}

static void calculate_scaling(
	Vertex* output,
	int buf_width, int buf_height,
	int texture_width, int texture_height,
	int window_width, int window_height,
	int scale_mode)
{
	float x_ratio = 1.0f / (float)window_width;
	float y_ratio = 1.0f / (float)window_height;
	float u_ratio = (float)texture_width / (float)buf_width;
	float v_ratio = (float)texture_height / (float)buf_height;

	//Box box = { 0, 0, window_width, window_height };
	//gen_normalized(output, &box, x_ratio, y_ratio, u_ratio, v_ratio);

	switch (scale_mode) {
		case ScaleMode_Stretch:
		{
			Box box = { 0, 0, window_width, window_height };
			gen_normalized(output, &box, x_ratio, y_ratio, u_ratio, v_ratio);
			break;
		}

		case ScaleMode_AspectRatioStretch:
		{
			float buffer_aspect = (float)buf_width / (float)buf_height;
			float win_aspect = (float)window_width / (float)(window_height);

			if (buffer_aspect > win_aspect) {
				int new_height = (int)(window_width / buffer_aspect);
				int offset = (new_height - window_height) / -2;

				Box box = { 0, offset, window_width, offset + new_height };
				gen_normalized(output, &box, x_ratio, y_ratio, u_ratio, v_ratio);
			} else {
				int new_width = (int)(window_height * buffer_aspect);
				int offset = (new_width - window_width) / -2;

				Box box = { offset, 0, offset + new_width, window_height };
				gen_normalized(output, &box, x_ratio, y_ratio, u_ratio, v_ratio);
			}

			break;
		}

		default:
			break;
	}
}


@implementation WindowViewController
-(void)mtkView:(nonnull MTKView *)view drawableSizeWillChange:(CGSize)size
{
	m_width = (int)size.width;
	m_height = (int)size.height;
	(void)view;
}


-(void)drawInMTKView:(nonnull MTKView *)view
{
    // Wait to ensure only MaxBuffersInFlight number of frames are getting proccessed
    //   by any stage in the Metal pipeline (App, Metal, Drivers, GPU, etc)
    dispatch_semaphore_wait(m_semaphore, DISPATCH_TIME_FOREVER);

    // Iterate through our Metal buffers, and cycle back to the first when we've written to MaxBuffersInFlight
    m_current_buffer = (m_current_buffer + 1) % MaxBuffersInFlight;

    // Calculate the number of bytes per row of our image.
    NSUInteger bytesPerRow = 4 * m_draw_parameters->buffer_stride;
    MTLRegion region = { { 0, 0, 0 },
    	{ m_draw_parameters->buffer_width,
    	  m_draw_parameters->buffer_height, 1 } };

    // Copy the bytes from our data object into the texture
    [m_draw_state[m_current_buffer].texture replaceRegion:region
                mipmapLevel:0 withBytes:m_draw_parameters->buffer bytesPerRow:bytesPerRow];

    // Update the vertex buffer
	calculate_scaling(
		m_draw_state[m_current_buffer].vertex_buffer.contents,
		m_draw_parameters->buffer_width, m_draw_parameters->buffer_height,
		m_draw_state[m_current_buffer].texture_width, m_draw_state[m_current_buffer].texture_height,
		m_width, m_height,
		m_draw_parameters->scale_mode);

    // Create a new command buffer for each render pass to the current drawable
    id<MTLCommandBuffer> commandBuffer = [g_command_queue commandBuffer];
    commandBuffer.label = @"minifb_command_buffer";

    // Add completion hander which signals _inFlightSemaphore when Metal and the GPU has fully
    //   finished processing the commands we're encoding this frame.  This indicates when the
    //   dynamic buffers filled with our vertices, that we're writing to this frame, will no longer
    //   be needed by Metal and the GPU, meaning we can overwrite the buffer contents without
    //   corrupting the rendering.
    __block dispatch_semaphore_t block_sema = m_semaphore;
    [commandBuffer addCompletedHandler:^(id<MTLCommandBuffer> buffer)
    {
    	(void)buffer;
        dispatch_semaphore_signal(block_sema);
    }];

    MTLRenderPassDescriptor* renderPassDescriptor = view.currentRenderPassDescriptor;

    if (renderPassDescriptor != nil)
    {
		renderPassDescriptor.colorAttachments[0].clearColor = MTLClearColorMake(1.0, 0.0, 0.0, 1.0);

        // Create a render command encoder so we can render into something
        id<MTLRenderCommandEncoder> renderEncoder =
        [commandBuffer renderCommandEncoderWithDescriptor:renderPassDescriptor];
        renderEncoder.label = @"minifb_command_encoder";

        // Set render command encoder state
        [renderEncoder setRenderPipelineState:g_pipeline_state];

        [renderEncoder setFragmentTexture:m_draw_state[m_current_buffer].texture atIndex:0];
        [renderEncoder setVertexBuffer:m_draw_state[m_current_buffer].vertex_buffer offset:0 atIndex:0];

        // Draw the vertices of our quads
        [renderEncoder drawPrimitives:MTLPrimitiveTypeTriangle
                          vertexStart:0
                          vertexCount:6];

        // We're done encoding commands
        [renderEncoder endEncoding];

        // Schedule a present once the framebuffer is complete using the current drawable
        [commandBuffer presentDrawable:view.currentDrawable];
    }

    // Finalize rendering here & push the command buffer to the GPU
    [commandBuffer commit];
}
@end

@implementation OSXWindowFrameView

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

-(void)updateTrackingAreas
{
    if(trackingArea != nil) {
        [self removeTrackingArea:trackingArea];
        [trackingArea release];
    }

    int opts = (NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways);
    trackingArea = [ [NSTrackingArea alloc] initWithRect:[self bounds]
                                            options:opts
                                            owner:self
                                            userInfo:nil];
    [self addTrackingArea:trackingArea];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mouseDown:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[0] = 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mouseUp:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[0] = 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)rightMouseDown:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[2] = 1;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)rightMouseUp:(NSEvent*)event
{
    (void)event;
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->mouse_state[2] = 0;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)scrollWheel:(NSEvent *)event
{
    OSXWindow* window = (OSXWindow*)[self window];
    window->shared_data->scroll_x = [event deltaX];
    window->shared_data->scroll_y = [event deltaY];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeKeyView
{
    return YES;
}

- (NSView *)nextValidKeyView
{
    return self;
}

- (NSView *)previousValidKeyView
{
    return self;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)acceptsFirstResponder
{
    return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)viewDidMoveToWindow
{
    [[NSNotificationCenter defaultCenter] addObserver:self
    selector:@selector(windowResized:) name:NSWindowDidResizeNotification
    object:[self window]];
}

- (void)viewDidEndLiveResize
{
	//NSRect originalFrame = [win frame];
	//NSRect contentRect = [NSWindow contentRectForFrameRect: originalFrame styleMask: NSWindowStyleMaskTitled];
    NSSize size = [self bounds].size;
	//NSSize size = [[self contentView] frame].size;
    OSXWindow* window = (OSXWindow*)[self window];

    int width = (int)size.width;
    int height = (int)size.height;

    //printf("resize %d\n");

    // if windows is resized we need to create new textures so we do that here and put the old textures in a
    // "to delete" queue and set a frame counter of 3 frames before the gets released

    if (window->shared_data) {
    	/*
		if ((width != (int)window->shared_data->width) ||
			(height != (int)window->shared_data->height)) {

			MTLTextureDescriptor* textureDescriptor = [[MTLTextureDescriptor alloc] init];

			// Indicate that each pixel has a blue, green, red, and alpha channel, where each channel is
			// an 8-bit unsigned normalized value (i.e. 0 maps to 0.0 and 255 maps to 1.0)
			textureDescriptor.pixelFormat = MTLPixelFormatBGRA8Unorm;

			// Set the pixel dimensions of the texture
			textureDescriptor.width = width;
			textureDescriptor.height = height;

			// Create the texture from the device by using the descriptor

			m_view_controller->m_width = width;
			m_view_controller->m_height = height;

			for (int i = 0; i < MaxBuffersInFlight; ++i) {
				m_view_controller->m_delayed_delete_count = 3;
				m_view_controller->m_delayed_delete_textures[i] = m_view_controller->m_texture_buffers[i];
				m_view_controller->m_texture_buffers[i] = [g_metal_device newTextureWithDescriptor:textureDescriptor];
			}
		}
		*/

        window->shared_data->width = width;
        window->shared_data->height = height;
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)dealloc
{
    [[NSNotificationCenter defaultCenter] removeObserver:self];
    [super dealloc];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)windowResized:(NSNotification *)notification
{
	(void)notification;
}

@end

