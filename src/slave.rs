use std::cell::RefCell;
use std::rc::Weak;

use syscall::error::{Error, Result, EINVAL, EPIPE, EWOULDBLOCK};
use syscall::flag::{F_GETFL, F_SETFL, O_ACCMODE, O_NONBLOCK};

use pty::Pty;
use resource::Resource;

/// Read side of a pipe
#[derive(Clone)]
pub struct PtySlave {
    pty: Weak<RefCell<Pty>>,
    flags: usize,
}

impl PtySlave {
    pub fn new(pty: Weak<RefCell<Pty>>, flags: usize) -> Self {
        PtySlave {
            pty: pty,
            flags: flags,
        }
    }
}

impl Resource for PtySlave {
    fn boxed_clone(&self) -> Box<Resource> {
        Box::new(self.clone())
    }

    fn pty(&self) -> Weak<RefCell<Pty>> {
        self.pty.clone()
    }

    fn flags(&self) -> usize {
        self.flags
    }

    fn path(&self, buf: &mut [u8]) -> Result<usize> {
        if let Some(pty_lock) = self.pty.upgrade() {
            pty_lock.borrow_mut().path(buf)
        } else {
            Err(Error::new(EPIPE))
        }
    }

    fn read(&self, buf: &mut [u8]) -> Result<usize> {
        if let Some(pty_lock) = self.pty.upgrade() {
            let mut pty = pty_lock.borrow_mut();

            let mut i = 0;

            while i < buf.len() && ! pty.mosi.is_empty() {
                buf[i] = pty.mosi.pop_front().unwrap();
                i += 1;
            }

            if i > 0 || self.flags & O_NONBLOCK == O_NONBLOCK {
                Ok(i)
            } else {
                Err(Error::new(EWOULDBLOCK))
            }
        } else {
            Ok(0)
        }
    }

    fn write(&self, buf: &[u8]) -> Result<usize> {
        if let Some(pty_lock) = self.pty.upgrade() {
            let mut vec = Vec::new();
            vec.push(0);
            vec.extend_from_slice(buf);

            let mut pty = pty_lock.borrow_mut();
            pty.miso.push_back(vec);

            Ok(buf.len())
        } else {
            Err(Error::new(EPIPE))
        }
    }

    fn sync(&self) -> Result<usize> {
        if let Some(pty_lock) = self.pty.upgrade() {
            let mut vec = Vec::new();
            vec.push(1);

            let mut pty = pty_lock.borrow_mut();
            pty.miso.push_back(vec);

            Ok(0)
        } else {
            Err(Error::new(EPIPE))
        }
    }

    fn fcntl(&mut self, cmd: usize, arg: usize) -> Result<usize> {
        match cmd {
            F_GETFL => Ok(self.flags),
            F_SETFL => {
                self.flags = (self.flags & O_ACCMODE) | (arg & ! O_ACCMODE);
                Ok(0)
            },
            _ => Err(Error::new(EINVAL))
        }
    }

    fn fevent(&self) -> Result<()> {
        Ok(())
    }

    fn fevent_count(&self) -> Option<usize> {
        if let Some(pty_lock) = self.pty.upgrade() {
            let pty = pty_lock.borrow();
            if ! pty.mosi.is_empty() {
                Some(pty.mosi.len())
            } else {
                None
            }
        } else {
            Some(0)
        }
    }
}