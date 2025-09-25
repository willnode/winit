//! # Wayland
//!
//! **Note:** Windows don't appear on Wayland until you draw/present to them.
//!
//! By default, Winit loads system libraries using `dlopen`. This can be
//! disabled by disabling the `"wayland-dlopen"` cargo feature.
//!
//! ## Client-side decorations
//!
//! Winit provides client-side decorations by default, but the behaviour can
//! be controlled with the following feature flags:
//!
//! * `wayland-csd-adwaita` (default).
//! * `wayland-csd-adwaita-crossfont`.
//! * `wayland-csd-adwaita-notitle`.
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr::NonNull;

use rwh_06::HandleError;
use sctk::reexports::csd_frame::WindowState;

use crate::event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder};
use crate::monitor::MonitorHandle;
pub use crate::window::Theme;
use crate::window::{Window as CoreWindow, WindowAttributes};

/// Additional methods on [`ActiveEventLoop`] that are specific to Wayland.
pub trait ActiveEventLoopExtWayland {
    /// True if the [`ActiveEventLoop`] uses Wayland.
    fn is_wayland(&self) -> bool;
}

impl ActiveEventLoopExtWayland for dyn ActiveEventLoop + '_ {
    #[inline]
    fn is_wayland(&self) -> bool {
        self.as_any().downcast_ref::<crate::platform_impl::wayland::ActiveEventLoop>().is_some()
    }
}

/// Additional methods on [`EventLoop`] that are specific to Wayland.
pub trait EventLoopExtWayland {
    /// True if the [`EventLoop`] uses Wayland.
    fn is_wayland(&self) -> bool;
}

impl EventLoopExtWayland for EventLoop {
    #[inline]
    fn is_wayland(&self) -> bool {
        self.event_loop.is_wayland()
    }
}

/// Additional methods on [`EventLoopBuilder`] that are specific to Wayland.
pub trait EventLoopBuilderExtWayland {
    /// Force using Wayland.
    fn with_wayland(&mut self) -> &mut Self;

    /// Whether to allow the event loop to be created off of the main thread.
    ///
    /// By default, the window is only allowed to be created on the main
    /// thread, to make platform compatibility easier.
    fn with_any_thread(&mut self, any_thread: bool) -> &mut Self;
}

impl EventLoopBuilderExtWayland for EventLoopBuilder {
    #[inline]
    fn with_wayland(&mut self) -> &mut Self {
        self.platform_specific.forced_backend = Some(crate::platform_impl::Backend::Wayland);
        self
    }

    #[inline]
    fn with_any_thread(&mut self, any_thread: bool) -> &mut Self {
        self.platform_specific.any_thread = any_thread;
        self
    }
}

pub struct XdgSurfaceHandle<'a> {
    raw: NonNull<c_void>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> XdgSurfaceHandle<'a> {
    /// Create a `XdgSurfaceHandle` from a [`NonNull<c_void>`]
    pub unsafe fn borrow_raw(raw: NonNull<c_void>) -> Self {
        Self { raw, _marker: PhantomData }
    }

    /// Get the underlying raw xdg_surface handle.
    pub fn as_raw(&self) -> NonNull<c_void> {
        self.raw
    }
}

pub trait HasXdgSurfaceHandle {
    fn xdg_surface_handle(&self) -> Result<XdgSurfaceHandle<'_>, HandleError>;
}

pub struct XdgToplevelHandle<'a> {
    raw: NonNull<c_void>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> XdgToplevelHandle<'a> {
    /// Create a `XdgToplevelHandle` from a [`NonNull<c_void>`]
    pub unsafe fn borrow_raw(raw: NonNull<c_void>) -> Self {
        Self { raw, _marker: PhantomData }
    }

    /// Get the underlying raw xdg_toplevel handle.
    pub fn as_raw(&self) -> NonNull<c_void> {
        self.raw
    }
}

pub trait HasXdgToplevelHandle {
    fn xdg_toplevel_handle(&self) -> Result<XdgToplevelHandle<'_>, HandleError>;
}

/// Additional methods on [`Window`] that are specific to Wayland.
///
/// [`Window`]: crate::window::Window
pub trait WindowExtWayland {
    fn xdg_surface_handle<'a>(&'a self) -> Option<&dyn HasXdgSurfaceHandle>;
    fn xdg_toplevel_handle<'a>(&'a self) -> Option<&dyn HasXdgToplevelHandle>;
    /// Get the state of the window
    fn window_state(&self) -> Option<WindowState>;
}

impl WindowExtWayland for dyn CoreWindow + '_ {
    fn xdg_surface_handle(&self) -> Option<&dyn HasXdgSurfaceHandle> {
        let window = self.as_any().downcast_ref::<crate::platform_impl::wayland::Window>();
        window.map(|w| w as &dyn HasXdgSurfaceHandle)
    }

    fn xdg_toplevel_handle(&self) -> Option<&dyn HasXdgToplevelHandle> {
        let window = self.as_any().downcast_ref::<crate::platform_impl::wayland::Window>();
        window.map(|w| w as &dyn HasXdgToplevelHandle)
    }

    fn window_state(&self) -> Option<WindowState> {
        let window = self.as_any().downcast_ref::<crate::platform_impl::wayland::Window>();
        window.and_then(|w| w.xdg_window_state())
    }
}

/// Additional methods on [`WindowAttributes`] that are specific to Wayland.
pub trait WindowAttributesExtWayland {
    /// Build window with the given name.
    ///
    /// The `general` name sets an application ID, which should match the `.desktop`
    /// file distributed with your program. The `instance` is a `no-op`.
    ///
    /// For details about application ID conventions, see the
    /// [Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#desktop-file-id)
    fn with_name(self, general: impl Into<String>, instance: impl Into<String>) -> Self;
}

impl WindowAttributesExtWayland for WindowAttributes {
    #[inline]
    fn with_name(mut self, general: impl Into<String>, instance: impl Into<String>) -> Self {
        self.platform_specific.name =
            Some(crate::platform_impl::ApplicationName::new(general.into(), instance.into()));
        self
    }
}

/// Additional methods on `MonitorHandle` that are specific to Wayland.
pub trait MonitorHandleExtWayland {
    /// Returns the inner identifier of the monitor.
    fn native_id(&self) -> u32;
}

impl MonitorHandleExtWayland for MonitorHandle {
    #[inline]
    fn native_id(&self) -> u32 {
        self.inner.native_identifier()
    }
}
