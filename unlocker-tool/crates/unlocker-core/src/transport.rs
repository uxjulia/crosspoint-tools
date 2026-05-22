//! Cross-platform transport for the helper RPC channel.
//!
//! Unix: AF_UNIX socket at /var/run/com.sofriendly.crosspoint.unlocker.helper.sock.
//! Windows: named pipe at \\.\pipe\com.sofriendly.crosspoint.unlocker.helper.
//!
//! The streams returned implement AsyncRead + AsyncWrite + Unpin so callers
//! can use tokio::io::split for owned half-streams.

use anyhow::Result;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub fn endpoint() -> String {
    #[cfg(unix)]
    {
        "/var/run/com.sofriendly.crosspoint.unlocker.helper.sock".to_string()
    }
    #[cfg(windows)]
    {
        r"\\.\pipe\com.sofriendly.crosspoint.unlocker.helper".to_string()
    }
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tokio::net::{UnixListener, UnixStream};

    pub struct Stream(pub(super) UnixStream);

    pub struct Listener(UnixListener);

    impl Listener {
        pub fn bind(endpoint: &str) -> Result<Self> {
            let _ = std::fs::remove_file(endpoint);
            if let Some(p) = std::path::Path::new(endpoint).parent() {
                std::fs::create_dir_all(p).ok();
            }
            let l = UnixListener::bind(endpoint)?;
            let mut perm = std::fs::metadata(endpoint)?.permissions();
            perm.set_mode(0o666);
            std::fs::set_permissions(endpoint, perm)?;
            Ok(Self(l))
        }

        pub async fn accept(&self) -> Result<Stream> {
            let (s, _) = self.0.accept().await?;
            Ok(Stream(s))
        }
    }

    pub async fn connect(endpoint: &str) -> Result<Stream> {
        Ok(Stream(UnixStream::connect(endpoint).await?))
    }

    impl AsyncRead for Stream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for Stream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_shutdown(cx)
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::Arc;
    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
    };
    use tokio::sync::Mutex;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

    pub enum Stream {
        Server(NamedPipeServer),
        Client(NamedPipeClient),
    }

    /// Owns a SECURITY_DESCRIPTOR + SECURITY_ATTRIBUTES used when creating the
    /// helper's named pipe. Without an explicit DACL the elevated helper would
    /// inherit a default descriptor that denies non-elevated processes — so the
    /// medium-IL Tauri app can't open the pipe and `ping()` fails with
    /// ERROR_ACCESS_DENIED.
    ///
    /// The DACL grants GENERIC_ALL to Local System, Built-in Administrators, and
    /// Authenticated Users. That mirrors the macOS unix socket (mode 0666: any
    /// local logged-in user can talk to the helper).
    struct PipeSecurity {
        descriptor: PSECURITY_DESCRIPTOR,
        attrs: Box<SECURITY_ATTRIBUTES>,
    }

    // The descriptor is allocated once and only read by CreateNamedPipe.
    unsafe impl Send for PipeSecurity {}
    unsafe impl Sync for PipeSecurity {}

    impl PipeSecurity {
        fn new() -> std::io::Result<Self> {
            let sddl: Vec<u16> = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;AU)\0"
                .encode_utf16()
                .collect();
            let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
            let ok = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1 as u32,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let attrs = Box::new(SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor as *mut c_void,
                bInheritHandle: 0,
            });
            Ok(PipeSecurity { descriptor, attrs })
        }

        fn as_ptr(&self) -> *mut c_void {
            (&*self.attrs as *const SECURITY_ATTRIBUTES as *mut SECURITY_ATTRIBUTES) as *mut c_void
        }
    }

    impl Drop for PipeSecurity {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.descriptor as _);
            }
        }
    }

    pub struct Listener {
        endpoint: String,
        next: Mutex<Option<NamedPipeServer>>,
        security: Arc<PipeSecurity>,
    }

    impl Listener {
        pub fn bind(endpoint: &str) -> Result<Self> {
            let security = Arc::new(PipeSecurity::new()?);
            // first_pipe_instance ensures we fail loudly if another helper is
            // already running, instead of silently sharing the name.
            let server = unsafe {
                ServerOptions::new()
                    .first_pipe_instance(true)
                    .create_with_security_attributes_raw(endpoint, security.as_ptr())?
            };
            Ok(Self {
                endpoint: endpoint.to_string(),
                next: Mutex::new(Some(server)),
                security,
            })
        }

        pub async fn accept(&self) -> Result<Stream> {
            let mut guard = self.next.lock().await;
            let server = guard
                .take()
                .ok_or_else(|| anyhow::anyhow!("listener not initialized"))?;
            server.connect().await?;
            // Spin up the next instance immediately so the next client can
            // connect while we're servicing this one. Same DACL.
            let next = unsafe {
                ServerOptions::new()
                    .create_with_security_attributes_raw(&self.endpoint, self.security.as_ptr())?
            };
            *guard = Some(next);
            Ok(Stream::Server(server))
        }
    }

    pub async fn connect(endpoint: &str) -> Result<Stream> {
        // ERROR_PIPE_BUSY (231) means the server is between instances. Retry
        // briefly — the helper recreates an instance immediately after accept.
        // ERROR_FILE_NOT_FOUND (2) happens before the helper has created its
        // first pipe instance — also worth a brief retry while UAC settles.
        const ERROR_FILE_NOT_FOUND: i32 = 2;
        const ERROR_PIPE_BUSY: i32 = 231;
        for _ in 0..50 {
            match ClientOptions::new().open(endpoint) {
                Ok(c) => return Ok(Stream::Client(c)),
                Err(e)
                    if matches!(
                        e.raw_os_error(),
                        Some(ERROR_PIPE_BUSY) | Some(ERROR_FILE_NOT_FOUND)
                    ) =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(anyhow::anyhow!("named pipe {endpoint} unreachable"))
    }

    impl AsyncRead for Stream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            match &mut *self {
                Stream::Server(s) => Pin::new(s).poll_read(cx, buf),
                Stream::Client(c) => Pin::new(c).poll_read(cx, buf),
            }
        }
    }

    impl AsyncWrite for Stream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            match &mut *self {
                Stream::Server(s) => Pin::new(s).poll_write(cx, buf),
                Stream::Client(c) => Pin::new(c).poll_write(cx, buf),
            }
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            match &mut *self {
                Stream::Server(s) => Pin::new(s).poll_flush(cx),
                Stream::Client(c) => Pin::new(c).poll_flush(cx),
            }
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            match &mut *self {
                Stream::Server(s) => Pin::new(s).poll_shutdown(cx),
                Stream::Client(c) => Pin::new(c).poll_shutdown(cx),
            }
        }
    }
}

pub use imp::{connect, Listener, Stream};
