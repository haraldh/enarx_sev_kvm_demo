use std::io;

#[derive(Clone, PartialEq, Debug)]
pub enum ErrorKind {
    OverlappingUserspaceMemRegionExists,
    MemRegionWithSlotAlreadyExists,
    NoMemRegionWithSlotFound,
    NoMemFree,
    MmapFailed,
    MadviseFailed,
    VMModeUnsupported,
    NoMappingForVirtualAddress,
    NoVirtualAddressAvailable,
    GuestCodeNotFound,
    Io(::std::io::ErrorKind),
    Str(&'static str),
    Generic,
}

impl ::std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            ErrorKind::Io(_) => write!(f, "IO error"),
            ErrorKind::OverlappingUserspaceMemRegionExists => {
                write!(f, "overlapping userspace_mem_region already exists")
            }
            ErrorKind::NoMemRegionWithSlotFound => {
                write!(f, "userspace_mem_region with the requested slot not found")
            }
            ErrorKind::MemRegionWithSlotAlreadyExists => write!(
                f,
                "userspace_mem_region with the requested slot already exists"
            ),
            ErrorKind::NoMemFree => write!(f, "no phys mem free"),
            ErrorKind::MmapFailed => write!(f, "mmap failed"),
            ErrorKind::MadviseFailed => write!(f, "madvise failed"),
            ErrorKind::VMModeUnsupported => write!(f, "VM mode currently unsupported"),
            ErrorKind::NoMappingForVirtualAddress => write!(f, "no mapping for virtual address"),
            ErrorKind::GuestCodeNotFound => write!(f, "guest code not found"),
            ErrorKind::NoVirtualAddressAvailable => {
                write!(f, "No vaddr of specified pages available")
            }
            ErrorKind::Generic => Ok(()),
            ErrorKind::Str(s) => write!(f, "{}", s),
        }
    }
}

impl From<&io::Error> for ErrorKind {
    fn from(e: &io::Error) -> Self {
        ErrorKind::Io(e.kind())
    }
}

impl From<&&'static str> for ErrorKind {
    fn from(e: &&'static str) -> Self {
        ErrorKind::Str(e)
    }
}

pub struct Error(
    pub ErrorKind,
    pub Option<Box<dyn std::error::Error + 'static + Send + Sync>>,
    pub Option<&'static str>,
);

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }
}

impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Error(e, None, None)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::error::Error as StdError;

        if let Some(ref o) = self.2 {
            std::fmt::Display::fmt(o, f)?;
        }

        std::fmt::Debug::fmt(&self.0, f)?;
        if let Some(e) = self.source() {
            std::fmt::Display::fmt("\nCaused by:\n", f)?;
            std::fmt::Debug::fmt(&e, f)?;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! map_context {
    () => {
        |e| $crate::context!(e, $crate::ErrorKind::from(&e))
    };
}

#[macro_export]
macro_rules! context {
    ( $k:expr ) => {{
        $crate::error::Error($k, None, Some(concat!(file!(), ":", line!(), ": ")))
    }};
    ( None, $k:expr ) => {{
        $crate::error::Error($k, None, Some(concat!(file!(), ":", line!(), ": ")))
    }};
    ( $e:path, $k:expr ) => {{
        $crate::error::Error(
            $k,
            Some(Box::from($e)),
            Some(concat!(file!(), ":", line!(), ": ")),
        )
    }};
}
