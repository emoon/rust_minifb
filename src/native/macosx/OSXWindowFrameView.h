#import <Cocoa/Cocoa.h>

@interface OSXWindowFrameView : NSView
{
	@public int scale;
	@public int width;
	@public int height;
	@public void* draw_buffer;
	@private NSTrackingArea* trackingArea;
}

@end

