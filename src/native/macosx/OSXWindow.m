#import "OSXWindow.h"
#import "OSXWindowFrameView.h"

@implementation OSXWindow

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

	[super setContentSize:newFrameSize];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

-(void)flagsChanged:(NSEvent *)event
{
	const uint32_t flags = [event modifierFlags];

	// Left Shift
	key_callback(rust_data, 0x38, flags == 0x20102 ? 1 : 0);

	// RightShift
	key_callback(rust_data, 0x3c, flags == 0x20104 ? 1 : 0);

	// Left Ctrl
	key_callback(rust_data, 0x3b, flags == 0x40101 ? 1 : 0);

	// Right Ctrl
	key_callback(rust_data, 0x3b, flags == 0x42101 ? 1 : 0);

	// Left Alt
	key_callback(rust_data, 0x3a, flags == 0x80120 ? 1 : 0);

	// Right Super
	key_callback(rust_data, 0x3d, flags == 0x80140  ? 1 : 0);

	// Left Super
	key_callback(rust_data, 0x37, flags == 0x100108 ? 1 : 0);

	// Right Super
	key_callback(rust_data, 0x36, flags == 0x100110 ? 1 : 0);

	[super flagsChanged:event];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyDown:(NSEvent *)event
{
	if (key_callback) {
		key_callback(rust_data, [event keyCode], 1);
	}

	[super keyDown:event];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyUp:(NSEvent *)event
{
	if (key_callback) {
		key_callback(rust_data, [event keyCode], 0);
	}

	[super keyDown:event];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mainWindowChanged:(NSNotification *)note
{
	void* window = [note object];

	if (window == self) {
		self->is_active = true;
	} else {
		self->is_active = false;
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)windowWillClose:(NSNotification *)notification
{
	should_close = true;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)windowShouldClose:(id)sender
{
	should_close = true;
	return TRUE;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentView:(NSView *)aView
{
	if ([childContentView isEqualTo:aView])
		return;

	NSRect bounds = [self frame];
	bounds.origin = NSZeroPoint;

	OSXWindowFrameView* frameView = [super contentView];
	if (!frameView)
	{
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

		frameView = [[[OSXWindowFrameView alloc] initWithFrame:bounds] autorelease];
		frameView->width = width;
		frameView->height = height;
		frameView->draw_buffer = draw_buffer;
		frameView->scale = scale;
		[super setContentView:frameView];
	}

	if (childContentView)
		[childContentView removeFromSuperview];

	NSRect t = [self contentRectForFrameRect:bounds];

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
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)onMenuPress:(id)sender
{
	int menu_id = (int)((NSButton*)sender).tag;
	self->active_menu_id = menu_id;
}

@end



