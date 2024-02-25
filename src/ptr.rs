pub struct Ptr<T> {
    ptr: *const T, //NonNull doesn't have "null", dangling can't be checked
}

impl<T> Ptr<T> {
    pub fn null() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
        }
    }
    pub fn new(ptr: *const T) -> Self {
        Self { ptr }
    }

    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }

    pub fn into_mut<'a>(self) -> &'a mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }

    pub fn into_ref<'a>(self) -> &'a T {
        unsafe { &*self.ptr }
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr as *mut T
    }
}

impl<T> Default for Ptr<T> {
    fn default() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
        }
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> core::ops::Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> core::ops::DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}
impl<T> core::fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Ptr({:?})", self.ptr)
    }
}

impl<T> core::fmt::Pointer for Ptr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:p}", self.ptr)
    }
}

impl<T> core::fmt::Display for Ptr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:p}", self.ptr)
    }
}

impl<T> core::cmp::PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> core::cmp::Eq for Ptr<T> {}

impl<T> core::cmp::PartialOrd for Ptr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.ptr.partial_cmp(&other.ptr)
    }
}

impl<T> core::cmp::Ord for Ptr<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

impl<T> core::hash::Hash for Ptr<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<T> core::ops::Add<usize> for Ptr<T> {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            ptr: self.ptr.wrapping_add(rhs),
        }
    }
}

impl<T> core::ops::AddAssign<usize> for Ptr<T> {
    fn add_assign(&mut self, rhs: usize) {
        self.ptr = self.ptr.wrapping_add(rhs);
    }
}

impl<T> core::ops::Sub<usize> for Ptr<T> {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            ptr: self.ptr.wrapping_sub(rhs),
        }
    }
}

impl<T> core::ops::SubAssign<usize> for Ptr<T> {
    fn sub_assign(&mut self, rhs: usize) {
        self.ptr = self.ptr.wrapping_sub(rhs);
    }
}
