#import "OSXWindowFrameView.h"

@implementation OSXWindowFrameView

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSRect)resizeRect
{
	const CGFloat resizeBoxSize = 16.0;
	const CGFloat contentViewPadding = 5.5;
	
	NSRect contentViewRect = [[self window] contentRectForFrameRect:[[self window] frame]];
	NSRect resizeRect = NSMakeRect(
		NSMaxX(contentViewRect) + contentViewPadding,
		NSMinY(contentViewRect) - resizeBoxSize - contentViewPadding,
		resizeBoxSize,
		resizeBoxSize);
	
	return resizeRect;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)drawRect:(NSRect)rect
{
	CGContextRef context = [[NSGraphicsContext currentContext] graphicsPort];

	CGColorSpaceRef space = CGColorSpaceCreateDeviceRGB();
	CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, draw_buffer, width * height * 4, NULL); 

	CGImageRef img = CGImageCreate(width, height, 8, 32, width * 4, space, kCGImageAlphaNoneSkipFirst | kCGBitmapByteOrder32Little, 
								   provider, NULL, false, kCGRenderingIntentDefault);

	CGColorSpaceRelease(space);
	CGDataProviderRelease(provider);

	CGContextDrawImage(context, CGRectMake(0, 0, width, height), img);

	CGImageRelease(img);
}

@end

