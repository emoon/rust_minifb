extern crate user;

pub fn open(name: &str, width: usize, height: usize) -> bool {
    let s = CString::new(name).unwrap();
		
	unsafe {
		let class = winapi::WNDCLASSEXA {
			cbSize: mem::size_of::<winapi::WNDCLASSEXW>() as winapi::UINT,
			style: winapi::CS_HREDRAW | winapi::CS_VREDRAW | winapi::CS_OWNDC,
			lpfnWndProc: Some(callback::callback),
			cbClsExtra: 0,
			cbWndExtra: 0,
			hInstance: kernel32::GetModuleHandleA(ptr::null()),
			hIcon: ptr::null_mut(),
			hCursor: ptr::null_mut(),
			hbrBackground: ptr::null_mut(),
			lpszMenuName: ptr::null(),
			lpszClassName: s.as_ptr(),
			hIconSm: ptr::null_mut(),
		};

		user32::RegisterClassExA(&class);

		let mut rect = winapi::RECT {
			left: 0, right: width as winapi::LONG,
			top: 0, bottom: height as winapi::LONG,
		}

		user32::AdjustWindowRect(&mut rect, winapi::WS_POPUP | winapi::WS_SYSMENU | winapi::WS_CAPTION, 0);

		rect.right -= rect.left;
		rect.bottom -= rect.top;

		let handle = user32::CreateWindowExA(0,
            s.as_ptr(), s.as_ptr(),
            winapi::WS_OVERLAPPEDWINDOW & ~winapi::WS_MAXIMIZEBOX & ~winapi::WS_TICKFRAME,
            winapi::CW_USEDEFAULT, winapi::CW_USEDEFAULT,
            rect.right, rect.bottom,
            ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut());

		if handle.is_null() {
			return false;
		}

		user32::ShowWindow(handle, winapi::SW_NORMAL);
}

