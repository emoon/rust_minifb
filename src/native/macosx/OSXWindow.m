#import "OSXWindow.h"
#import "OSXWindowFrameView.h"

@implementation OSXWindow

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (id)initWithContentRect:(NSRect)contentRect
	styleMask:(NSUInteger)windowStyle
	backing:(NSBackingStoreType)bufferingType
	defer:(BOOL)deferCreation
{
	self = [super
		initWithContentRect:contentRect
		styleMask:windowStyle
		backing:bufferingType
		defer:deferCreation];
	if (self)
	{
		[self setOpaque:YES];
		[self setBackgroundColor:[NSColor clearColor]];
		
		[[NSNotificationCenter defaultCenter]
			addObserver:self
			selector:@selector(mainWindowChanged:)
			name:NSWindowDidBecomeMainNotification
			object:self];
		
		[[NSNotificationCenter defaultCenter]
			addObserver:self
			selector:@selector(mainWindowChanged:)
			name:NSWindowDidResignMainNotification
			object:self];
	}
	return self;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)dealloc
{
	[[NSNotificationCenter defaultCenter]
		removeObserver:self];
	[super dealloc];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentSize:(NSSize)newSize
{
	NSSize sizeDelta = newSize;
	NSSize childBoundsSize = [childContentView bounds].size;
	sizeDelta.width -= childBoundsSize.width;
	sizeDelta.height -= childBoundsSize.height;
	
	OSXWindowFrameView *frameView = [super contentView];
	NSSize newFrameSize = [frameView bounds].size;
	newFrameSize.width += sizeDelta.width;
	newFrameSize.height += sizeDelta.height;

	printf("conten size\n");
	
	[super setContentSize:newFrameSize];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mainWindowChanged:(NSNotification *)aNotification
{
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentView:(NSView *)aView
{
	if ([childContentView isEqualTo:aView])
	{
		return;
	}
	
	NSRect bounds = [self frame];
	bounds.origin = NSZeroPoint;

	printf("view size\n");

	OSXWindowFrameView* frameView = [super contentView];
	if (!frameView)
	{
		frameView = [[[OSXWindowFrameView alloc] initWithFrame:bounds] autorelease];
		frameView->width = width; 
		frameView->height = height; 
		frameView->draw_buffer = draw_buffer; 
		frameView->scale = scale;
		[super setContentView:frameView];
	}
	
	if (childContentView)
		[childContentView removeFromSuperview];

	childContentView = aView;
	[childContentView setFrame:[self contentRectForFrameRect:bounds]];
	[childContentView setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
	[frameView addSubview:childContentView];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSView *)contentView
{
	return childContentView;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeKeyWindow
{
	return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeMainWindow
{
	return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSRect)contentRectForFrameRect:(NSRect)windowFrame
{
	windowFrame.origin = NSZeroPoint;
	return NSInsetRect(windowFrame, 0, 0);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

+ (NSRect)frameRectForContentRect:(NSRect)windowContentRect styleMask:(NSUInteger)windowStyle
{
	return NSInsetRect(windowContentRect, 0, 0);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)updateSize 
{
	OSXWindowFrameView* frameView = [super contentView];
	if (frameView)
	{
		frameView->width = width; 
		frameView->height = height; 
		frameView->draw_buffer = draw_buffer; 
		frameView->scale = scale;
	}

	printf("UpdateSize %d %d - %d\n", width, height, scale);
}

@end
