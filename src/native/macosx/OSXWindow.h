#import <Cocoa/Cocoa.h>
#include "shared_data.h"

#define MAX_MENUS 512

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct Menu
{
	const char* name;
	NSMenu* menu;
	NSMenuItem* menu_item;
} Menu;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct MenuData
{
	Menu menus[MAX_MENUS];
	int menu_count;
} MenuData;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct MenuDesc {
	char name[512];
	struct MenuDesc* sub_menu;
	int menu_id;
	int key;
	int special_key;
	int modifier;
	int modifier_mac;
	int enabled;
} MenuDesc;

void build_submenu(NSMenu* menu, MenuDesc* desc);

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

@interface OSXWindow : NSWindow
{
	NSView* childContentView;
	@public void (*key_callback)(void* user_data, int key, int state);
	@public int width;
	@public int height;
	@public int scale;
	@public void* draw_buffer;
	@public void* rust_data;
	@public SharedData* shared_data;
	@public bool should_close;
	@public MenuData* menu_data;
}



@end
