use std::{cell::RefCell, fmt, rc::Rc};

pub struct RefCount<T>(pub(super) Rc<RefCell<T>>);

pub struct Weak<T>(pub(super) std::rc::Weak<RefCell<T>>);

pub type Ref<'a, T> = std::cell::Ref<'a, T>;

pub type RefMut<'a, T> = std::cell::RefMut<'a, T>;

impl<T> RefCount<T> {
    pub fn new(inner: T) -> Self {
        Self(Rc::new(RefCell::new(inner)))
    }

    pub fn clone(this: &Self) -> Self {
        Self(Rc::clone(&this.0))
    }

    pub fn downgrade(&self) -> Weak<T> {
        Weak(Rc::downgrade(&self.0))
    }

    pub fn get(&self) -> Ref<'_, T> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }
}

impl<T> Weak<T> {
    pub fn upgrade(&self) -> Option<RefCount<T>> {
        self.0.upgrade().map(RefCount)
    }
}

impl<T: fmt::Debug> fmt::Debug for RefCount<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
