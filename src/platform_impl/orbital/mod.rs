#![cfg(target_os = "redox")]

use std::fs::{File, OpenOptions};
use std::io::{Read, Result, Write};
use std::num::{NonZeroU16, NonZeroU32};
use std::os::fd::AsRawFd;
use std::{fmt, mem, slice, str};

use libredox::data::TimeSpec;
use redox_event::{user_data, EventFlags};
use smol_str::SmolStr;

pub(crate) use self::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy, OwnedDisplayHandle};
use crate::dpi::{PhysicalPosition, PhysicalSize};
use crate::keyboard::Key;
mod event_loop;

pub use self::window::Window;
mod window;

pub(crate) use crate::cursor::{
    NoCustomCursor as PlatformCustomCursor, NoCustomCursor as PlatformCustomCursorSource,
};
pub(crate) use crate::icon::NoIcon as PlatformIcon;

redox_event::user_data! {
    pub enum EventSource {
        Orbital,
        Time,
    }
}

struct RedoxSocket {
    fd: File,
}

impl RedoxSocket {
    fn orbital(properties: &WindowProperties<'_>) -> Result<Self> {
        Self::open_raw(&format!("{properties}"))
    }

    // Paths should be checked to ensure they are actually sockets and not normal files. If a
    // non-socket path is used, it could cause read and write to not function as expected. For
    // example, the seek would change in a potentially unpredictable way if either read or write
    // were called at the same time by multiple threads.
    fn open_raw(path: &str) -> Result<Self> {
        let fd = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self { fd })
    }

    fn fd(&self) -> usize {
        self.fd.as_raw_fd() as usize
    }

    fn read(&self, buf: &mut [u8]) -> Result<()> {
        (&self.fd).read_exact(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<()> {
        (&self.fd).write_all(buf)
    }

    fn fpath<'a>(&self, buf: &'a mut [u8]) -> Result<&'a str> {
        let count = libredox::call::fpath(self.fd(), buf)?;
        str::from_utf8(&buf[..count])
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
    }
}

pub struct TimeSocket(RedoxSocket);

impl TimeSocket {
    fn open() -> Result<Self> {
        RedoxSocket::open_raw("/scheme/time/4").map(Self)
    }

    // Read current time.
    fn current_time(&self) -> Result<TimeSpec> {
        let mut timespec: libredox::data::TimeSpec = unsafe { mem::zeroed() };
        let timespec_bytes = unsafe {
            slice::from_raw_parts_mut(
                &mut timespec as *mut _ as *mut u8,
                mem::size_of::<TimeSpec>(),
            )
        };
        self.0.read(timespec_bytes)?;
        Ok(timespec)
    }

    // Write a timeout.
    fn timeout(&self, timespec: &TimeSpec) -> Result<()> {
        let timespec_bytes = unsafe {
            slice::from_raw_parts(timespec as *const _ as *const u8, mem::size_of::<TimeSpec>())
        };
        self.0.write(timespec_bytes)
    }

    // Wake immediately.
    fn wake(&self) -> Result<()> {
        // Writing a default TimeSpec will always trigger a time event.
        let timespec: TimeSpec = unsafe { mem::zeroed() };
        self.timeout(&timespec)
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PlatformSpecificEventLoopAttributes {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId {
    fd: u64,
}

impl WindowId {
    pub const fn dummy() -> Self {
        WindowId { fd: u64::MAX }
    }
}

impl From<WindowId> for u64 {
    fn from(id: WindowId) -> Self {
        id.fd
    }
}

impl From<u64> for WindowId {
    fn from(fd: u64) -> Self {
        Self { fd }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceId;

impl DeviceId {
    pub const fn dummy() -> Self {
        DeviceId
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FingerId;

impl FingerId {
    pub const fn dummy() -> Self {
        FingerId
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlatformSpecificWindowAttributes;

struct WindowProperties<'a> {
    flags: &'a str,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    title: &'a str,
}

impl<'a> WindowProperties<'a> {
    fn new(path: &'a str) -> Self {
        // /scheme/orbital/flags/x/y/w/h/t
        let mut parts = path.splitn(6, '/');
        let flags = parts.next().unwrap_or("");
        let x = parts.next().map_or(0, |part| part.parse::<i32>().unwrap_or(0));
        let y = parts.next().map_or(0, |part| part.parse::<i32>().unwrap_or(0));
        let w = parts.next().map_or(0, |part| part.parse::<u32>().unwrap_or(0));
        let h = parts.next().map_or(0, |part| part.parse::<u32>().unwrap_or(0));
        let title = parts.next().unwrap_or("");
        Self { flags, x, y, w, h, title }
    }
}

impl<'a> fmt::Display for WindowProperties<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}/{}/{}/{}",
            std::env::var("ORBITAL_DISPLAY").unwrap_or("/scheme/orbital".to_string()),
            self.flags,
            self.x,
            self.y,
            self.w,
            self.h,
            self.title
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonitorHandle;

impl MonitorHandle {
    pub fn name(&self) -> Option<String> {
        None
    }

    pub fn position(&self) -> Option<PhysicalPosition<i32>> {
        None
    }

    pub fn scale_factor(&self) -> f64 {
        1.0 // TODO
    }

    pub fn current_video_mode(&self) -> Option<VideoModeHandle> {
        // (it is guaranteed to support 32 bit color though)
        Some(VideoModeHandle { monitor: self.clone() })
    }

    pub fn video_modes(&self) -> impl Iterator<Item = VideoModeHandle> {
        self.current_video_mode().into_iter()
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VideoModeHandle {
    monitor: MonitorHandle,
}

impl VideoModeHandle {
    pub fn size(&self) -> PhysicalSize<u32> {
        // TODO
        PhysicalSize::default()
    }

    pub fn bit_depth(&self) -> Option<NonZeroU16> {
        None
    }

    pub fn refresh_rate_millihertz(&self) -> Option<NonZeroU32> {
        // TODO
        None
    }

    pub fn monitor(&self) -> MonitorHandle {
        self.monitor.clone()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {
    pub key_without_modifiers: Key,
    pub text_with_all_modifiers: Option<SmolStr>,
}
