use winit::{self, window::Window};

// A wrapper around the winit window that allows us to implement the trait necessary for enabling
// the winit <-> conrod conversion functions.
pub struct WindowRef<'a>(&'a Window);

// Implement the `WinitWindow` trait for `WindowRef` to allow for generating compatible conversion
// functions.
impl<'a> conrod_winit::WinitWindow for WindowRef<'a> {
    fn get_inner_size(&self) -> Option<(u32, u32)> {
        Some(Window::inner_size(&self.0).into())
    }
    fn hidpi_factor(&self) -> f32 {
        Window::scale_factor(&self.0) as _
    }
}

conrod_winit::v021_conversion_fns!();
