use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// lri 目录的链接状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// 不存在
    Missing,
    /// 指向其它目标的 Junction
    Junction(PathBuf),
    /// 普通目录
    Directory,
    /// 其它链接或非目录
    Other,
}

/// 确保 link 是指向 target 的 Junction
pub fn ensure(link: &Path, target: &Path) -> Result<()> {
    match platform::state(link)? {
        State::Missing => {}
        State::Junction(current) if same(&current, target) => {
            println!("[lri] Junction 已存在: {}", link.display());
            return Ok(());
        }
        State::Junction(current) => {
            println!(
                "[lri] 重建 Junction: {} ({})",
                link.display(),
                current.display()
            );
            platform::remove(link)?;
        }
        State::Directory => bail!("{} 是普通目录，禁止自动删除", link.display()),
        State::Other => bail!("{} 不是本工具创建的 Junction，禁止自动删除", link.display()),
    }

    platform::create(link, target)?;
    println!(
        "[lri] 已创建 Junction: {} → {}",
        link.display(),
        target.display()
    );
    Ok(())
}

/// 删除本工具创建的 Junction
pub fn remove_link(link: &Path, target: &Path) -> Result<()> {
    match platform::state(link)? {
        State::Missing => Ok(()),
        State::Junction(current) if same(&current, target) => {
            platform::remove(link)?;
            println!("[lri] 已删除 Junction: {}", link.display());
            Ok(())
        }
        State::Junction(_) | State::Directory | State::Other => {
            bail!("{} 不是本工具目标的 Junction，禁止删除", link.display())
        }
    }
}

/// 比较两个路径是否指向同一位置
fn same(left: &Path, right: &Path) -> bool {
    let resolve = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    resolve(left)
        .to_string_lossy()
        .eq_ignore_ascii_case(&resolve(right).to_string_lossy())
}

#[cfg(windows)]
mod platform {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result, bail};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_NOT_A_REPARSE_POINT, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateDirectoryW, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    use super::State;

    /// FSCTL_SET_REPARSE_POINT
    const SET_REPARSE_POINT: u32 = 0x0009_00a4;
    /// FSCTL_GET_REPARSE_POINT
    const GET_REPARSE_POINT: u32 = 0x0009_00a8;
    /// IO_REPARSE_TAG_MOUNT_POINT
    const TAG_MOUNT_POINT: u32 = 0xa000_0003;
    /// 设备路径前缀
    const DEVICE_PREFIX: &str = r"\??\";
    /// 读取重解析点用的缓冲区大小
    const BUFFER_SIZE: usize = 16 * 1024;

    /// 读取链接状态
    pub fn state(path: &Path) -> Result<State> {
        let Ok(metadata) = std::fs::symlink_metadata(path) else {
            return Ok(State::Missing);
        };

        let handle = open(path, GENERIC_READ)?;
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                handle,
                GET_REPARSE_POINT,
                std::ptr::null(),
                0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        let error = unsafe { GetLastError() };
        unsafe { CloseHandle(handle) };

        if result == 0 {
            if error == ERROR_NOT_A_REPARSE_POINT {
                return Ok(ordinary(&metadata));
            }

            bail!("无法读取 {} 的重解析点 (错误码 {error})", path.display());
        }

        if read_u32(&buffer, 0) != TAG_MOUNT_POINT {
            return Ok(State::Other);
        }

        let start = 16 + read_u16(&buffer, 8) as usize;
        let length = read_u16(&buffer, 10) as usize;
        let name = utf16(&buffer[start..start + length]);
        let target = name.strip_prefix(DEVICE_PREFIX).unwrap_or(&name);
        Ok(State::Junction(PathBuf::from(target)))
    }

    /// 创建 Junction
    pub fn create(link: &Path, target: &Path) -> Result<()> {
        let buffer = mount_point(target)?;
        let wide_link = wide(link.as_os_str());
        if unsafe { CreateDirectoryW(wide_link.as_ptr(), std::ptr::null()) } == 0 {
            let error = unsafe { GetLastError() };
            bail!("无法创建目录 {} (错误码 {error})", link.display());
        }

        let handle = open(link, GENERIC_WRITE)?;
        let result = unsafe {
            DeviceIoControl(
                handle,
                SET_REPARSE_POINT,
                buffer.as_ptr().cast(),
                buffer.len() as u32,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let error = unsafe { GetLastError() };
        unsafe { CloseHandle(handle) };

        if result == 0 {
            let _ = std::fs::remove_dir(link);
            bail!("创建 Junction 失败: {} (错误码 {error})", link.display());
        }

        Ok(())
    }

    /// 删除链接本身，不影响目标目录
    pub fn remove(path: &Path) -> Result<()> {
        std::fs::remove_dir(path).with_context(|| format!("无法删除 {}", path.display()))
    }

    /// 构建 mount point 的重解析点数据
    fn mount_point(target: &Path) -> Result<Vec<u8>> {
        let substitute = wide(OsStr::new(&format!("{DEVICE_PREFIX}{}", target.display())));
        let print = wide(target.as_os_str());
        let substitute_bytes = (substitute.len() + 1) * 2;
        let print_bytes = (print.len() + 1) * 2;
        if substitute_bytes > u16::MAX as usize || print_bytes > u16::MAX as usize {
            bail!("路径过长: {}", target.display());
        }

        let data_length = 8 + substitute_bytes + print_bytes;
        let mut buffer = vec![0u8; 8 + data_length];
        buffer[0..4].copy_from_slice(&TAG_MOUNT_POINT.to_ne_bytes());
        buffer[4..6].copy_from_slice(&(data_length as u16).to_ne_bytes());
        buffer[8..10].copy_from_slice(&0u16.to_ne_bytes());
        buffer[10..12].copy_from_slice(&(substitute_bytes as u16).to_ne_bytes());
        buffer[12..14].copy_from_slice(&(substitute_bytes as u16).to_ne_bytes());
        buffer[14..16].copy_from_slice(&(print_bytes as u16).to_ne_bytes());

        let mut offset = 16;
        for unit in substitute.iter().copied().chain(once(0u16)) {
            buffer[offset..offset + 2].copy_from_slice(&unit.to_ne_bytes());
            offset += 2;
        }
        for unit in print.iter().copied().chain(once(0u16)) {
            buffer[offset..offset + 2].copy_from_slice(&unit.to_ne_bytes());
            offset += 2;
        }

        Ok(buffer)
    }

    /// 以打开重解析点本身的方式打开目录
    fn open(path: &Path, access: u32) -> Result<HANDLE> {
        let wide_path = wide(path.as_os_str());
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let error = unsafe { GetLastError() };
            bail!("无法打开 {} (错误码 {error})", path.display());
        }

        Ok(handle)
    }

    fn ordinary(metadata: &std::fs::Metadata) -> State {
        if metadata.is_dir() {
            State::Directory
        } else {
            State::Other
        }
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(once(0)).collect()
    }

    fn utf16(bytes: &[u8]) -> String {
        let units = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_ne_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    }

    fn read_u16(buffer: &[u8], offset: usize) -> u16 {
        u16::from_ne_bytes([buffer[offset], buffer[offset + 1]])
    }

    fn read_u32(buffer: &[u8], offset: usize) -> u32 {
        u32::from_ne_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ])
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    use anyhow::{Result, bail};

    use super::State;

    pub fn state(_path: &Path) -> Result<State> {
        bail!("lri Junction 仅支持 Windows")
    }

    pub fn create(_link: &Path, _target: &Path) -> Result<()> {
        bail!("lri Junction 仅支持 Windows")
    }

    pub fn remove(_link: &Path) -> Result<()> {
        bail!("lri Junction 仅支持 Windows")
    }
}
