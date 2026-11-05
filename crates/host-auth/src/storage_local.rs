//! One-object local transfers. No source pathnames or directory capabilities cross this channel.
use anyhow::{ensure, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::File,
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, Interest},
    net::UnixStream,
};

const MAX_FRAME: usize = 16 * 1024;
const MARKER: u8 = 1;

pub async fn write_frame<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= MAX_FRAME, "Local Storage frame is too large");
    stream.write_u32(bytes.len() as u32).await?;
    stream.write_all(&bytes).await?;
    Ok(())
}

pub async fn read_frame<T: DeserializeOwned>(stream: &mut UnixStream) -> Result<T> {
    let length = stream.read_u32().await? as usize;
    ensure!(length <= MAX_FRAME, "Local Storage frame is too large");
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// The first byte carries at most one read-only source descriptor. Subsequent frames/body use Tokio.
pub async fn send_source(stream: &UnixStream, source: Option<&File>) -> Result<()> {
    if let Some(source) = source {
        let access = unsafe { libc::fcntl(source.as_raw_fd(), libc::F_GETFL) };
        ensure!(
            access >= 0 && access & libc::O_ACCMODE == libc::O_RDONLY,
            "Local import requires a read-only source descriptor"
        );
    }
    loop {
        stream.writable().await?;
        let result = stream.try_io(Interest::WRITABLE, || {
            let mut marker = [MARKER];
            let mut iov = libc::iovec {
                iov_base: marker.as_mut_ptr().cast(),
                iov_len: 1,
            };
            // usize storage provides the alignment required by cmsghdr.
            let space = unsafe { libc::CMSG_SPACE(mem::size_of::<libc::c_int>() as _) } as usize;
            let mut control = vec![0usize; space.div_ceil(mem::size_of::<usize>())];
            let mut msg: libc::msghdr = unsafe { mem::zeroed() };
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            if let Some(file) = source {
                msg.msg_control = control.as_mut_ptr().cast();
                msg.msg_controllen = space as _;
                unsafe {
                    let cmsg = libc::CMSG_FIRSTHDR(&msg);
                    (*cmsg).cmsg_level = libc::SOL_SOCKET;
                    (*cmsg).cmsg_type = libc::SCM_RIGHTS;
                    (*cmsg).cmsg_len = libc::CMSG_LEN(mem::size_of::<libc::c_int>() as _) as _;
                    std::ptr::write_unaligned(
                        libc::CMSG_DATA(cmsg).cast::<libc::c_int>(),
                        file.as_raw_fd(),
                    );
                }
            }
            let n = unsafe { libc::sendmsg(stream.as_raw_fd(), &msg, 0) };
            if n == 1 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
        match result {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            result => return Ok(result?),
        }
    }
}

pub async fn receive_source(stream: &UnixStream) -> Result<Option<File>> {
    loop {
        stream.readable().await?;
        let result = stream.try_io(Interest::READABLE, || {
            let mut marker = [0u8];
            let mut iov = libc::iovec {
                iov_base: marker.as_mut_ptr().cast(),
                iov_len: 1,
            };
            let space =
                unsafe { libc::CMSG_SPACE((mem::size_of::<libc::c_int>() * 4) as _) } as usize;
            let mut control = vec![0usize; space.div_ceil(mem::size_of::<usize>())];
            let mut msg: libc::msghdr = unsafe { mem::zeroed() };
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = control.as_mut_ptr().cast();
            msg.msg_controllen = space as _;
            let n = unsafe { libc::recvmsg(stream.as_raw_fd(), &mut msg, 0) };
            if n < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut files = Vec::new();
            unsafe {
                let mut cmsg = libc::CMSG_FIRSTHDR(&msg);
                while !cmsg.is_null() {
                    if (*cmsg).cmsg_level == libc::SOL_SOCKET
                        && (*cmsg).cmsg_type == libc::SCM_RIGHTS
                    {
                        let size =
                            ((*cmsg).cmsg_len as usize).saturating_sub(libc::CMSG_LEN(0) as usize);
                        for i in 0..size / mem::size_of::<libc::c_int>() {
                            let fd = std::ptr::read_unaligned(
                                libc::CMSG_DATA(cmsg).cast::<libc::c_int>().add(i),
                            );
                            let owned = OwnedFd::from_raw_fd(fd);
                            if libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
                                return Err(io::Error::last_os_error());
                            }
                            files.push(owned);
                        }
                    }
                    cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
                }
            }
            if n != 1
                || marker[0] != MARKER
                || msg.msg_flags & libc::MSG_CTRUNC != 0
                || files.len() > 1
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid local Storage descriptor",
                ));
            }
            if let Some(file) = files.last() {
                let access = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
                if access < 0 || access & libc::O_ACCMODE != libc::O_RDONLY {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Local import requires a read-only source descriptor",
                    ));
                }
            }
            Ok(files.pop().map(File::from))
        });
        match result {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            result => return result.context("Read local Storage source"),
        }
    }
}

use core_api::{
    StorageBackendPermission as Permission, StorageLocalEndpoint,
    StorageLocalOperation as Operation, StorageLocalReply as Reply, StorageLocalRequest,
};
use std::{collections::HashMap, path::PathBuf, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::Mutex;
pub type Reader = Pin<Box<dyn tokio::io::AsyncRead + Send + Unpin>>;
pub type Blob = core_api::backend::Blob;

#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    async fn put(&self, owner: &str, blob: &Blob, input: Reader) -> Result<()>;
    async fn import(&self, owner: &str, blob: &Blob, source: File) -> Result<()>;
    async fn source(&self, owner: &str, id: &str) -> Result<(Blob, File)>;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Grant {
    scope: String,
    owner: String,
    blob: Blob,
    permission: Permission,
    expires: u64,
}

pub struct Server {
    _directory: tempfile::TempDir,
    socket: PathBuf,
    repository: Arc<dyn Repository>,
    grants: Mutex<HashMap<String, Grant>>,
    reads: crate::storage_grants::ReadGrants,
}
impl Server {
    pub async fn start(repository: Arc<dyn Repository>) -> Result<Arc<Self>> {
        let directory = tempfile::Builder::new().prefix("meow-storage-").tempdir()?;
        let socket = directory.path().join("data.sock");
        let listener = tokio::net::UnixListener::bind(&socket)?;
        let server = Arc::new(Self {
            _directory: directory,
            socket,
            repository,
            grants: Mutex::new(HashMap::new()),
            reads: crate::storage_grants::ReadGrants::default(),
        });
        let weak = Arc::downgrade(&server);
        tokio::spawn(async move {
            loop {
                if weak.strong_count() == 0 {
                    break;
                }
                match tokio::time::timeout(Duration::from_secs(30), listener.accept()).await {
                    Ok(Ok((stream, _))) => {
                        let Some(server) = weak.upgrade() else {
                            break;
                        };
                        tokio::spawn(async move {
                            let _ = tokio::time::timeout(
                                Duration::from_secs(120),
                                server.serve(stream),
                            )
                            .await;
                        });
                    }
                    Ok(Err(_)) => break,
                    Err(_) => continue,
                }
            }
        });
        Ok(server)
    }
    pub async fn issue(
        &self,
        scope: &str,
        owner: &str,
        blob: Blob,
        permission: Permission,
        expires: u64,
    ) -> Result<StorageLocalEndpoint> {
        let grant = Grant {
            scope: scope.into(),
            owner: owner.into(),
            blob,
            permission,
            expires,
        };
        let token = if permission == Permission::Read {
            self.reads.issue(scope, grant, expires).await?
        } else {
            let mut grants = self.grants.lock().await;
            grants.retain(|_, g| g.expires > now());
            ensure!(grants.len() < 4096, "Too many local Storage grants");
            let token = crate::signatures::random_id();
            grants.insert(token.clone(), grant);
            token
        };
        Ok(StorageLocalEndpoint {
            socket: self.socket.to_string_lossy().into(),
            token,
        })
    }
    pub async fn revoke(&self, scope: &str) {
        self.reads.revoke(scope).await;
        self.grants.lock().await.retain(|_, g| g.scope != scope);
    }
    pub async fn retire_writes(&self, owner: &str, id: &str) {
        self.grants.lock().await.retain(|_, g| {
            g.owner != owner || g.blob.id != id || g.permission != Permission::Write
        });
    }
    async fn serve(&self, mut stream: UnixStream) -> Result<()> {
        let result = self.execute(&mut stream).await;
        if let Err(error) = result {
            let error = error
                .downcast_ref::<core_api::StorageError>()
                .cloned()
                .unwrap_or_else(|| {
                    core_api::StorageError::new(
                        core_api::StorageErrorCode::Unavailable,
                        error.to_string(),
                    )
                    .uncertain()
                });
            write_frame(&mut stream, &Reply::Error { error }).await?;
        }
        Ok(())
    }
    async fn execute(&self, stream: &mut UnixStream) -> Result<()> {
        let source = receive_source(stream).await?;
        let request: StorageLocalRequest = read_frame(stream).await?;
        let grant = if request.operation == Operation::Read {
            self.reads.verify::<Grant>(&request.token).await.context(
                core_api::StorageError::new(
                    core_api::StorageErrorCode::Unauthorized,
                    "Invalid local Storage grant",
                ),
            )?
        } else {
            let mut grants = self.grants.lock().await;
            let g = grants
                .get(&request.token)
                .filter(|g| g.expires > now())
                .cloned()
                .context(core_api::StorageError::new(
                    core_api::StorageErrorCode::Unauthorized,
                    "Invalid local Storage grant",
                ))?;
            ensure!(
                g.permission == Permission::Write,
                core_api::StorageError::new(
                    core_api::StorageErrorCode::Unauthorized,
                    "Local Storage permission denied"
                )
            );
            grants.remove(&request.token);
            g
        };
        ensure!(
            source.is_some() == (request.operation == Operation::Import),
            "Local Storage source does not match operation"
        );
        if request.operation == Operation::Read {
            let (blob, source) = self.repository.source(&grant.owner, &grant.blob.id).await?;
            ensure!(
                blob == grant.blob,
                core_api::StorageError::new(
                    core_api::StorageErrorCode::Integrity,
                    "Local Storage declaration changed"
                )
            );
            let range = match request.range {
                Some(ref value) => Some(byte_range(value, blob.size_bytes).context(
                    core_api::StorageError::new(
                        core_api::StorageErrorCode::InvalidInput,
                        "Invalid byte range",
                    ),
                )?),
                None => None,
            };
            write_frame(
                stream,
                &Reply::Ready {
                    blob: blob.clone(),
                    range,
                },
            )
            .await?;
            let mut source = tokio::fs::File::from_std(source);
            let (start, length) = range
                .map(|(a, b)| (a, b - a + 1))
                .unwrap_or((0, blob.size_bytes));
            tokio::io::AsyncSeekExt::seek(&mut source, std::io::SeekFrom::Start(start)).await?;
            // After Ready, this is a raw byte stream. Closing reports read failure; never append JSON to media.
            let _ = tokio::io::copy(&mut source.take(length), stream).await;
        } else {
            write_frame(
                stream,
                &Reply::Ready {
                    blob: grant.blob.clone(),
                    range: None,
                },
            )
            .await?;
            if let Some(source) = source {
                self.repository
                    .import(&grant.owner, &grant.blob, source)
                    .await?;
            } else {
                // Move the read half into an owned Reader; keep the write half for the receipt.
                let source = stream.as_raw_fd();
                let fd = unsafe { libc::fcntl(source, libc::F_DUPFD_CLOEXEC, 0) };
                ensure!(fd >= 0, "Cannot duplicate local Storage stream");
                let input = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
                input.set_nonblocking(true)?;
                let input = UnixStream::from_std(input)?;
                self.repository
                    .put(&grant.owner, &grant.blob, Box::pin(input))
                    .await?;
            }
            write_frame(stream, &Reply::Done).await?;
        }
        Ok(())
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn byte_range(value: &str, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    let (a, b) = value.strip_prefix("bytes=")?.split_once('-')?;
    let (a, b) = if a.is_empty() {
        let n: u64 = b.parse().ok()?;
        if n == 0 {
            return None;
        }
        (size.saturating_sub(n), size - 1)
    } else {
        (
            a.parse().ok()?,
            if b.is_empty() {
                size - 1
            } else {
                b.parse::<u64>().ok()?.min(size - 1)
            },
        )
    };
    (a <= b && a < size).then_some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn local_sources_transfer_descriptors_without_paths_and_frames_are_bounded() {
        let mut temporary = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temporary, b"source").unwrap();
        let source = File::open(temporary.path()).unwrap();
        let (mut sender, mut receiver) = UnixStream::pair().unwrap();
        send_source(&sender, Some(&source)).await.unwrap();
        write_frame(
            &mut sender,
            &StorageLocalRequest {
                token: "object".into(),
                operation: Operation::Import,
                range: None,
            },
        )
        .await
        .unwrap();
        let mut imported = receive_source(&receiver).await.unwrap().unwrap();
        let request: StorageLocalRequest = read_frame(&mut receiver).await.unwrap();
        assert_eq!(request.operation, Operation::Import);
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut imported, &mut bytes).unwrap();
        assert_eq!(bytes, b"source");
        sender.write_u32(MAX_FRAME as u32 + 1).await.unwrap();
        assert!(read_frame::<StorageLocalRequest>(&mut receiver)
            .await
            .is_err());
    }
    struct NeverRead;
    #[async_trait::async_trait]
    impl Repository for NeverRead {
        async fn put(&self, _: &str, _: &Blob, _: Reader) -> Result<()> {
            panic!("unauthorized write");
        }
        async fn import(&self, _: &str, _: &Blob, _: File) -> Result<()> {
            panic!("unauthorized import");
        }
        async fn source(&self, _: &str, _: &str) -> Result<(Blob, File)> {
            panic!("unauthorized read");
        }
    }
    #[tokio::test]
    async fn unused_read_alternatives_do_not_exhaust_local_write_authorization() {
        let server = Server::start(Arc::new(NeverRead)).await.unwrap();
        let blob = Blob {
            id: "object".into(),
            size_bytes: 5,
            sha256: "a".repeat(64),
        };
        let mut first = None;
        for _ in 0..5000 {
            let grant = server
                .issue(
                    "space",
                    "owner",
                    blob.clone(),
                    Permission::Read,
                    now() + 60000,
                )
                .await
                .unwrap();
            first.get_or_insert(grant);
        }
        assert!(server.grants.lock().await.is_empty());
        server
            .issue("space", "owner", blob, Permission::Write, now() + 60000)
            .await
            .unwrap();
        let token = first.unwrap().token;
        assert!(server.reads.verify::<Grant>(&token).await.is_ok());
        assert!(server.reads.verify::<Grant>(&token).await.is_ok());
        server.revoke("space").await;
        assert!(server.reads.verify::<Grant>(&token).await.is_err());
        assert!(server.grants.lock().await.is_empty());
    }

    #[tokio::test]
    async fn local_streams_reject_unknown_or_wrong_operation_grants_before_content() {
        let server = Server::start(Arc::new(NeverRead)).await.unwrap();
        let blob = Blob {
            id: "object".into(),
            size_bytes: 5,
            sha256: "a".repeat(64),
        };
        let grant = server
            .issue("space", "owner", blob, Permission::Read, now() + 60000)
            .await
            .unwrap();
        for token in ["invalid".to_string(), grant.token] {
            let mut stream = UnixStream::connect(&grant.socket).await.unwrap();
            send_source(&stream, None).await.unwrap();
            write_frame(
                &mut stream,
                &StorageLocalRequest {
                    token,
                    operation: Operation::Write,
                    range: None,
                },
            )
            .await
            .unwrap();
            assert!(
                matches!(read_frame::<Reply>(&mut stream).await.unwrap(),Reply::Error{error} if error.code==core_api::StorageErrorCode::Unauthorized)
            );
        }
    }
}
