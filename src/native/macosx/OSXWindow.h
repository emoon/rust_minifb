#import <Cocoa/Cocoa.h>

@interface OSXWindow : NSWindow
{
	NSView* childContentView;
	@public int width;
	@public int height;
	@public int scale;
	@public void* draw_buffer;
}

@end
