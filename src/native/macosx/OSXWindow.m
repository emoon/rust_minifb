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
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyDown:(NSEvent *)event
{
	if (key_callback) {
		key_callback(rust_data, [event keyCode], 1);
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyUp:(NSEvent *)event
{
	if (key_callback) {
		key_callback(rust_data, [event keyCode], 0);
	}
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mainWindowChanged:(NSNotification *)aNotification
{
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
	int id = (int)((NSButton*)sender).tag;
	printf("menu id %d\n", id);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void build_submenu(NSMenu* menu, MenuDesc* desc)
{
	[menu removeAllItems];

	while (desc->menu_id != -2)
	{
		NSString* name = [NSString stringWithUTF8String: desc->name];

		printf("building submenu %s\n", desc->name);

		if (desc->menu_id == -1)
		{
			[menu addItem:[NSMenuItem separatorItem]];
		}
		/*
		else if (desc->id == EDITOR_MENU_SUB_MENU)
		{
			MyMenuItem* newItem = [[MyMenuItem allocWithZone:[NSMenu menuZone]] initWithTitle:name action:NULL keyEquivalent:@""];
			NSMenu* newMenu = [[NSMenu allocWithZone:[NSMenu menuZone]] initWithTitle:name];
			[newItem setSubmenu:newMenu];
			[newMenu release];
			[menu addItem:newItem];
			[newItem release];
		}
		*/
		else
		{
			int mask = 0;
			NSMenuItem* newItem = [[NSMenuItem alloc] initWithTitle:name action:@selector(onMenuPress:) keyEquivalent:@""];
			[newItem setTag:desc->menu_id];

			/*
			if (desc->macMod & EMGUI_KEY_COMMAND)
				mask |= NSCommandKeyMask;
			if (desc->macMod & EMGUI_KEY_SHIFT)
				mask |= NSShiftKeyMask;
			if (desc->macMod & EMGUI_KEY_CTRL)
				mask |= NSControlKeyMask; 
			if (desc->macMod & EMGUI_KEY_ALT)
				mask |= NSAlternateKeyMask; 
			*/

			NSString* key = 0; //convertKeyCodeToString(desc->key);

			if (key)
			{
				[newItem setKeyEquivalentModifierMask: mask];
				[newItem setKeyEquivalent:key];
			}
			else
			{
				//fprintf(stderr, "Unable to map keyboard shortcut for %s\n", desc->name);
			}

			[newItem setOnStateImage: newItem.offStateImage];
			[menu addItem:newItem];
			[newItem release];
		}

		desc++;
	}
}	

@end
