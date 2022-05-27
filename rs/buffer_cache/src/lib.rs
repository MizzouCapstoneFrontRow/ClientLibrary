use std::{collections::{VecDeque, HashMap, HashSet}, sync::{Mutex, Arc, Weak}, borrow::{Borrow, BorrowMut}, ops::{Deref, DerefMut}};

pub trait Buffer: Default {
    fn new_buffer(capacity: usize) -> Self;
    fn clear(&mut self);
}

pub struct BufferCache<Buf: Buffer> {
    buffers: Mutex<VecDeque<Buf>>,
    default_capacity: usize,
}

pub struct BufferWrapper<Buf: Buffer> {
    buffer: Buf,
    cache: Weak<BufferCache<Buf>>,
}

impl<Buf: Buffer> Drop for BufferWrapper<Buf> {
    fn drop(&mut self) {
        let cache = match self.cache.upgrade(){
            Some(cache) => cache,
            None => return,
        };
        let mut buffers = cache.buffers.lock().unwrap();
        self.buffer.clear();
        buffers.push_back(std::mem::take(&mut self.buffer));
    }
}

impl<T> Buffer for Vec<T> {
    fn new_buffer(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn clear(&mut self) {
        self.clear()
    }
    
}

impl<T> Buffer for VecDeque<T> {
    fn new_buffer(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl<K, V> Buffer for HashMap<K, V> {
    fn new_buffer(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl<T> Buffer for HashSet<T> {
    fn new_buffer(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl Buffer for String {
    fn new_buffer(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl<Buf: Buffer> BufferCache<Buf> {
    pub fn new(default_capacity: usize) -> Arc<Self> {
        Arc::new(Self { buffers: Mutex::new(VecDeque::with_capacity(128)), default_capacity })
    }

    pub fn try_get_buffer(self: &Arc<Self>) -> Option<BufferWrapper<Buf>> {
        let buffer = self.buffers.lock().unwrap().pop_front()?;
        Some(BufferWrapper {
            buffer,
            cache: Arc::downgrade(self),
        })
    }

    pub fn get_buffer(self: &Arc<Self>) -> BufferWrapper<Buf> {
        self.try_get_buffer().unwrap_or_else(||
            BufferWrapper {
                buffer: Buf::new_buffer(self.default_capacity),
                cache: Arc::downgrade(self),
            }
        )
    }
}



impl<Buf: Buffer> BufferWrapper<Buf> {
    /// Removed this buffer from the cache permanently
    pub fn into_inner(mut self) -> Buf {
        // The drop implementation will be a no-op if cache.upgrade() is None,
        self.cache = Weak::new();
        // so it won't try to add the default buffer we swap with back into the cache.
        std::mem::take(&mut self.buffer)
    }
}


impl<T: ?Sized, Buf: Buffer + AsRef<T>> AsRef<T> for BufferWrapper<Buf> {
    fn as_ref(&self) -> &T {
        self.buffer.as_ref()
    }
}

impl<Buf: Buffer> Borrow<Buf> for BufferWrapper<Buf> {
    fn borrow(&self) -> &Buf {
        &self.buffer
    }
}

impl<Buf: Buffer> Deref for BufferWrapper<Buf> {
    type Target = Buf;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T: ?Sized, Buf: Buffer + AsMut<T>> AsMut<T> for BufferWrapper<Buf> {
    fn as_mut(&mut self) -> &mut T {
        self.buffer.as_mut()
    }
}

impl<Buf: Buffer> BorrowMut<Buf> for BufferWrapper<Buf> {
    fn borrow_mut(&mut self) -> &mut Buf {
        &mut self.buffer
    }
}

impl<Buf: Buffer> DerefMut for BufferWrapper<Buf> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
